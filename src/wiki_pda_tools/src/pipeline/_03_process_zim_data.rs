use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

use crate::utils::article_processing::{self, ArticleProcessor};
use crate::utils::checkpoints;
use crate::utils::compression;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;
use crate::utils::txt_file_processing::{self, SortMode};

#[derive(Default, Debug, Clone)]
struct LangMetrics {
    pub articles_processed: u64,
    pub articles_skipped_not_in_db: u64,
    pub articles_failed_read: u64,
    pub articles_failed_process: u64,
    pub dur_db: std::time::Duration,
    pub dur_zim_read: std::time::Duration,
    pub dur_process: std::time::Duration,
    pub dur_compress: std::time::Duration,
    pub dur_overhead: std::time::Duration,
    pub dur_wall_time: std::time::Duration,
}

#[derive(Default, Debug)]
struct ZimMetrics {
    pub total_zim_files_processed: u64,
    pub metrics_per_lang: HashMap<String, LangMetrics>,
    pub program_total_duration: std::time::Duration,
    pub final_offset: u64,
    pub max_valid_offset: u64,
}

impl ZimMetrics {
    fn merge(&mut self, other: Self) {
        self.total_zim_files_processed += other.total_zim_files_processed;

        for (lang, metrics) in other.metrics_per_lang {
            let entry = self.metrics_per_lang.entry(lang).or_default();
            entry.articles_processed += metrics.articles_processed;
            entry.articles_skipped_not_in_db += metrics.articles_skipped_not_in_db;
            entry.articles_failed_read += metrics.articles_failed_read;
            entry.articles_failed_process += metrics.articles_failed_process;
            entry.dur_db += metrics.dur_db;
            entry.dur_zim_read += metrics.dur_zim_read;
            entry.dur_process += metrics.dur_process;
            entry.dur_compress += metrics.dur_compress;
            entry.dur_overhead += metrics.dur_overhead;
            entry.dur_wall_time += metrics.dur_wall_time;
        }
    }

    pub fn make_summary(&self) -> String {
        let new_data_mb =
            (self.final_offset.saturating_sub(self.max_valid_offset)) as f64 / 1_048_576.0;
        let total_data_mb = self.final_offset as f64 / 1_048_576.0;

        let mut total_db = std::time::Duration::ZERO;
        let mut total_read = std::time::Duration::ZERO;
        let mut total_proc = std::time::Duration::ZERO;
        let mut total_comp = std::time::Duration::ZERO;
        let mut total_over = std::time::Duration::ZERO;
        let mut total_wall = std::time::Duration::ZERO;

        for m in self.metrics_per_lang.values() {
            total_db += m.dur_db;
            total_read += m.dur_zim_read;
            total_proc += m.dur_process;
            total_comp += m.dur_compress;
            total_over += m.dur_overhead;
            total_wall += m.dur_wall_time;
        }

        let total_wall_secs = total_wall.as_secs_f64().max(0.0001);
        let pct_db = (total_db.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_read = (total_read.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_proc = (total_proc.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_comp = (total_comp.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_over = (total_over.as_secs_f64() / total_wall_secs) * 100.0;

        let mut summary = format!(
            "==================================================\n\
             =                ZIM PROCESSING SUMMARY          =\n\
             ==================================================\n\
             Total duration:                {:.2?}\n\
             Total ZIM files processed:     {} (Lifetime)\n\
             Binary data written (New):     {:.2} MB\n\
             Binary data total size:        {:.2} MB\n\
             \n\
             Worker Thread Time Profiling (Cumulative):\n\
             - Database Lookup (incl String): {:<10.2?} ({:05.2}%)\n\
             - ZIM Read & Decode:           {:<10.2?} ({:05.2}%)\n\
             - Article Processing:          {:<10.2?} ({:05.2}%)\n\
             - ZSTD Compression:            {:<10.2?} ({:05.2}%)\n\
             - Sync/Channel/Overhead:       {:<10.2?} ({:05.2}%)\n\
             --------------------------------------------------\n\
             - Total Worker Wall Time:      {:.2?}\n\
             \n\
             Breakdown by Language:\n",
            self.program_total_duration,
            self.total_zim_files_processed,
            new_data_mb,
            total_data_mb,
            total_db,
            pct_db,
            total_read,
            pct_read,
            total_proc,
            pct_proc,
            total_comp,
            pct_comp,
            total_over,
            pct_over,
            total_wall
        );

        let mut langs: Vec<&String> = self.metrics_per_lang.keys().collect();
        langs.sort();

        for lang in langs {
            let m = self.metrics_per_lang.get(lang).unwrap();
            let lang_wall_secs = m.dur_wall_time.as_secs_f64().max(0.0001);

            writeln!(&mut summary, "   - {:<18}", lang).unwrap();
            writeln!(
                &mut summary,
                "       Successfully processed:    {}",
                m.articles_processed
            )
            .unwrap();
            writeln!(
                &mut summary,
                "       Skipped (Not QID Lookup table): {}",
                m.articles_skipped_not_in_db
            )
            .unwrap();

            if m.articles_failed_read > 0 || m.articles_failed_process > 0 {
                writeln!(
                    &mut summary,
                    "       ERROR (Read/UTF8):         {}",
                    m.articles_failed_read
                )
                .unwrap();
                writeln!(
                    &mut summary,
                    "       Error (Processing):        {}",
                    m.articles_failed_process
                )
                .unwrap();
            }

            writeln!(
                &mut summary,
                "       Time Profile: DB: {:.1}% | Read: {:.1}% | Proc: {:.1}% | Zstd: {:.1}%",
                (m.dur_db.as_secs_f64() / lang_wall_secs) * 100.0,
                (m.dur_zim_read.as_secs_f64() / lang_wall_secs) * 100.0,
                (m.dur_process.as_secs_f64() / lang_wall_secs) * 100.0,
                (m.dur_compress.as_secs_f64() / lang_wall_secs) * 100.0
            )
            .unwrap();
        }

        writeln!(
            &mut summary,
            "=================================================="
        )
        .unwrap();
        summary
    }
}

enum WorkItem {
    Article(ProcessedArticle),
    ZimFinished(String, ZimMetrics),
    Anomaly {
        qid: String,
        lang: String,
        title: String,
        error_msg: String,
        raw_content: String,
    },
}

struct ProcessedArticle {
    qid: String,
    lang: String,
    title: String,
    binary_data: Vec<u8>,
}

pub fn process_directories(
    settings: &Settings,
    max_test_articles: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut completed_zims = HashSet::new();
    let mut checkpoint_data = String::new();

    match checkpoints::checkpoint_exists(settings, 3) {
        checkpoints::CheckpointState::ExistsEmpty => {
            println!("Checkpoint found: Zim processing has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::ExistsWithData(data) => {
            for line in data.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    completed_zims.insert(trimmed.to_string());
                }
            }
            checkpoint_data = data;
            if !checkpoint_data.ends_with('\n') && !checkpoint_data.is_empty() {
                checkpoint_data.push('\n');
            }
        }
        checkpoints::CheckpointState::ExistsInBadState(i) => {
            let _ = checkpoints::clear_checkpoints(settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".into());
        }
        checkpoints::CheckpointState::DoesNotExist => (),
    }

    let program_start_time = Instant::now();

    let languages_to_include: HashSet<String> = settings
        .database_content
        .language_to_include
        .iter()
        .map(|lang| lang.as_str().to_string())
        .collect();

    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let data_dir = PathBuf::from(&settings.paths.data_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);
    let log_dir = PathBuf::from(&settings.paths.log_dir);

    let qid_idx_unsorted_txt_path =
        tmp_dir.join(constants::QID_INDEX_TXT.replace(".txt", "_unsorted.txt"));
    let qid_idx_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let zstd_dictionary_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);
    let content_bin_path = bin_dir.join(constants::CONTENT_BIN);
    let anomalies_path = log_dir.join("anomalies.jsonl");

    let text_delimiter = settings.other.text_delimiter.clone();
    let text_delim_str = text_delimiter.as_str();
    let ram_limit_mb = settings.performance.ram_limit_mb;

    let previously_completed_count = completed_zims.len() as u64;
    let mut zim_files_with_size: Vec<(PathBuf, u64, String)> = Vec::new();

    let dir = data_dir.join("wiki");
    let raw_pattern = &settings.match_patterns.wikipedia_zim_file_match_pattern;
    let regex_str = raw_pattern.replace("{lang}", "(?P<lang>[a-zA-Z-]+)");
    let re = Regex::new(&regex_str).expect("Invalid Regex Pattern in config");

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            let path_str = path.to_string_lossy().to_string();

            if completed_zims.contains(&path_str) {
                continue;
            }

            if let Some(captures) = re.captures(&file_name) {
                if let Some(lang_match) = captures.name("lang") {
                    let lang = lang_match.as_str().to_string();
                    if languages_to_include.contains(&lang) {
                        if let Ok(metadata) = entry.metadata() {
                            zim_files_with_size.push((path, metadata.len(), lang));
                        }
                    }
                }
            }
        }
    }

    let total_zims_to_process = zim_files_with_size.len();
    zim_files_with_size.sort_by_key(|&(_, size, _)| size);
    let shared_queue = Arc::new(Mutex::new(zim_files_with_size));

    if total_zims_to_process == 0 {
        println!("No (new) ZIM files to process found.");
        return Ok(());
    }

    let multi_progress = Arc::new(MultiProgress::new());
    multi_progress
        .println(format!(
            "Found pending ZIM files: {}",
            total_zims_to_process
        ))
        .unwrap();

    let (tx, rx) = mpsc::sync_channel::<WorkItem>(10_000);
    let mut max_valid_offset: u64 = 0;

    if let Ok(qid_idx_file) = File::open(&qid_idx_unsorted_txt_path) {
        let reader = BufReader::new(qid_idx_file);
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split(text_delim_str).collect();
            if parts.len() == 5 {
                if let (Ok(offset), Ok(length)) = (parts[2].parse::<u64>(), parts[3].parse::<u64>())
                {
                    let end_of_article = offset + length;
                    if end_of_article > max_valid_offset {
                        max_valid_offset = end_of_article;
                    }
                }
            }
        }
    }

    if let Ok(bin_file) = OpenOptions::new().write(true).open(&content_bin_path) {
        bin_file
            .set_len(max_valid_offset)
            .expect("Error repairing the .bin file");
    }

    let settings_writer_clone = settings.clone();
    let mp_writer_clone = Arc::clone(&multi_progress);
    let qid_idx_unsorted_path_clone = qid_idx_unsorted_txt_path.clone();

    let writer_thread = thread::spawn(move || {
        run_writer_thread(
            rx,
            content_bin_path,
            qid_idx_unsorted_path_clone,
            anomalies_path,
            max_valid_offset,
            previously_completed_count,
            total_zims_to_process,
            mp_writer_clone,
            settings_writer_clone,
            checkpoint_data,
        )
    });

    let thread_count = settings.performance.thread_count;
    let worker_thread_count = std::cmp::max(1, thread_count - 2);

    println!("Starting {worker_thread_count} worker threads...");
    println!("Opening Key-Value Store for multi-threaded access...");

    let shared_db = Arc::new(sitelinks_lookup::open_sitelinks_db(settings));

    thread::scope(|s| {
        for worker_id in 0..worker_thread_count {
            let tx_clone = tx.clone();
            let queue_clone = Arc::clone(&shared_queue);
            let mp_clone = Arc::clone(&multi_progress);
            let db_clone = Arc::clone(&shared_db);
            let dict_path_clone = zstd_dictionary_bin_path.clone();

            s.spawn(move || {
                run_worker_thread(
                    worker_id,
                    settings,
                    max_test_articles,
                    tx_clone,
                    queue_clone,
                    mp_clone,
                    db_clone,
                    dict_path_clone,
                );
            });
        }
    });

    drop(tx);

    let (mut global_metrics, final_offset) = writer_thread.join().expect("Writer thread crashed");

    txt_file_processing::external_merge_sort(
        qid_idx_unsorted_txt_path.to_str().unwrap(),
        qid_idx_txt_path.to_str().unwrap(),
        SortMode::XId,
        ram_limit_mb,
        thread_count,
        &text_delimiter,
    )
    .expect("Failed to sort QID Index");

    global_metrics.program_total_duration = program_start_time.elapsed();
    global_metrics.final_offset = final_offset;
    global_metrics.max_valid_offset = max_valid_offset;

    let summary = global_metrics.make_summary();

    logs::write_summary_to_log(&summary, settings, true, constants::ZIM_PROCESSING_LOG)?;

    checkpoints::make_checkpoint(settings, 3, "zim_processing", None).map_err(|e| {
        format!(
            "Finished zim processing, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_writer_thread(
    rx: mpsc::Receiver<WorkItem>,
    content_bin_path: PathBuf,
    qid_idx_unsorted_path: PathBuf,
    anomalies_path: PathBuf,
    max_valid_offset: u64,
    previously_completed_count: u64,
    total_zims_to_process: usize,
    mp: Arc<MultiProgress>,
    settings: Settings,
    mut checkpoint_data: String,
) -> (ZimMetrics, u64) {
    let content_bin_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&content_bin_path)
        .unwrap();
    let qid_idx_unsorted_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&qid_idx_unsorted_path)
        .unwrap();
    let anomalies_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&anomalies_path)
        .unwrap();

    let mut bin_writer = BufWriter::new(content_bin_file);
    let mut idx_writer = BufWriter::new(qid_idx_unsorted_file);
    let mut anomalies_writer = BufWriter::new(anomalies_file);

    let mut current_offset = max_valid_offset;
    let mut processed_zims_this_run = 0;

    let mut global_metrics = ZimMetrics {
        total_zim_files_processed: previously_completed_count,
        ..Default::default()
    };

    for msg in rx {
        match msg {
            WorkItem::Article(article) => {
                let data_len = article.binary_data.len() as u64;
                let d = &settings.other.text_delimiter;
                bin_writer.write_all(&article.binary_data).unwrap();
                writeln!(
                    idx_writer,
                    "{0}{1}{2}{1}{3}{1}{4}{1}{5}",
                    article.qid, d, article.lang, current_offset, data_len, article.title
                )
                .unwrap();
                current_offset += data_len;
            }
            WorkItem::Anomaly {
                qid,
                lang,
                title,
                error_msg,
                raw_content,
            } => {
                let json_line = serde_json::json!({
                    "qid": qid,
                    "lang": lang,
                    "title": title,
                    "error_msg": error_msg,
                    "raw_content": raw_content
                });

                writeln!(anomalies_writer, "{}", json_line.to_string()).unwrap();
            }
            WorkItem::ZimFinished(zim_path, local_metrics) => {
                bin_writer.flush().unwrap();
                idx_writer.flush().unwrap();
                anomalies_writer.flush().unwrap();

                checkpoint_data.push_str(&zim_path);
                checkpoint_data.push('\n');

                if let Err(e) = checkpoints::make_checkpoint(
                    &settings,
                    3,
                    "zim_processing",
                    Some(&checkpoint_data),
                ) {
                    eprintln!("Warning: Failed to save progression checkpoint: {}", e);
                }

                global_metrics.merge(local_metrics);
                global_metrics.total_zim_files_processed += 1;
                processed_zims_this_run += 1;

                let percentage =
                    (processed_zims_this_run as f64 / total_zims_to_process as f64) * 100.0;
                mp.println(format!(
                    "ZIM finished: {} | Progress: {}/{} ({:.1}%)",
                    Path::new(&zim_path).file_name().unwrap().to_string_lossy(),
                    processed_zims_this_run,
                    total_zims_to_process,
                    percentage
                ))
                .unwrap();
            }
        }
    }

    bin_writer.flush().unwrap();
    idx_writer.flush().unwrap();
    anomalies_writer.flush().unwrap();

    (global_metrics, current_offset)
}

#[allow(clippy::too_many_arguments)]
fn run_worker_thread(
    worker_id: usize,
    settings: &Settings,
    max_test_articles: Option<usize>,
    tx: mpsc::SyncSender<WorkItem>,
    queue: Arc<Mutex<Vec<(PathBuf, u64, String)>>>,
    mp: Arc<MultiProgress>,
    db: Arc<redb::Database>,
    dict_path: PathBuf,
) {
    let read_txn = db.begin_read().expect("Could not begin read transaction");
    let table = read_txn
        .open_table(sitelinks_lookup::SITELINKS_TABLE)
        .expect("Table not found");

    let encoder_dict = compression::load_zstd_encoder_dictionary(
        &dict_path,
        settings.performance.zstd_compression_level,
    )
    .expect("Failed to load zstd encoder dictionary");

    let mut search_key = String::with_capacity(256);
    let article_processor = article_processing::DefaultArticleProcessor;

    loop {
        let next_zim = {
            let mut q = queue.lock().unwrap();
            q.pop()
        };

        let (zim_path, _, lang) = match next_zim {
            Some(data) => data,
            None => break,
        };

        let path_string = zim_path.to_string_lossy().to_string();
        let zim_file = zim::Zim::new(&zim_path).expect("Could not open/parse ZIM file");

        let pb = mp.add(ProgressBar::new(zim_file.header.article_count as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("T{} | {} (KV-Store)", worker_id, lang));

        let mut metrics = LangMetrics::default();
        let mut num_articles_looked_at = 0;
        let zim_wall_clock = Instant::now();

        for direntry_result in zim_file.iterate_by_urls() {
            pb.inc(1);

            let direntry = match direntry_result {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            if !matches!(
                direntry.namespace,
                zim::Namespace::Articles | zim::Namespace::UserContent
            ) {
                continue;
            }
            if matches!(direntry.target, Some(zim::Target::Redirect(_))) {
                continue;
            }

            let t_db_lookup = Instant::now();
            let (qid_opt, primary_title) = sitelinks_lookup::lookup_qid_from_sitelinks(
                &table,
                &mut search_key,
                settings,
                &lang,
                direntry.title.as_str(),
                &direntry.url,
            );
            metrics.dur_db += t_db_lookup.elapsed();

            let qid = match qid_opt {
                Some(q) => q,
                None => {
                    metrics.articles_skipped_not_in_db += 1;
                    continue;
                }
            };

            let t_read = Instant::now();
            let content = match zim_file.entry_content(&direntry) {
                Ok(Some(c)) => c,
                _ => {
                    metrics.articles_failed_read += 1;
                    continue;
                }
            };

            let article_text = match content
                .with(|bytes| unsafe { std::str::from_utf8_unchecked(bytes).to_string() })
            {
                Ok(text) => text,
                Err(_) => {
                    metrics.articles_failed_read += 1;
                    continue;
                }
            };
            metrics.dur_zim_read += t_read.elapsed();

            let t_process = Instant::now();
            let process_result = article_processor.process(
                &qid,
                &article_text,
                &table,
                &mut search_key,
                settings,
                &lang,
            );
            metrics.dur_process += t_process.elapsed();
            let raw_bin_data = match process_result {
                Ok(data) => data,
                Err(e) => {
                    metrics.articles_failed_process += 1;
                    tx.send(WorkItem::Anomaly {
                        qid: qid.clone(),
                        lang: lang.clone(),
                        title: primary_title.clone(),
                        error_msg: e.to_string(),
                        raw_content: article_text,
                    })
                    .unwrap();
                    continue;
                }
            };

            let t_compress = Instant::now();
            let compressed_data = compression::compress_data_zstd(
                &raw_bin_data,
                &encoder_dict,
                settings.performance.zstd_window_size_kb,
            )
            .expect("Failed to compress article with zstd");
            metrics.dur_compress += t_compress.elapsed();

            tx.send(WorkItem::Article(ProcessedArticle {
                qid,
                lang: lang.clone(),
                title: primary_title,
                binary_data: compressed_data,
            }))
            .expect("Writer thread died, could not send article");

            metrics.articles_processed += 1;
            num_articles_looked_at += 1;

            if num_articles_looked_at % 1000 == 0 {
                let dur_total_measured = metrics.dur_db
                    + metrics.dur_zim_read
                    + metrics.dur_process
                    + metrics.dur_compress;
                let total = dur_total_measured.as_secs_f64().max(0.0001);
                let pct_db = (metrics.dur_db.as_secs_f64() / total) * 100.0;
                let pct_read = (metrics.dur_zim_read.as_secs_f64() / total) * 100.0;
                let pct_proc = (metrics.dur_process.as_secs_f64() / total) * 100.0;
                let pct_zstd = (metrics.dur_compress.as_secs_f64() / total) * 100.0;

                pb.set_message(format!(
                    "T{} | {} (KV-Store) [DB: {:02.0}% | Read: {:02.0}% | Proc: {:02.0}% | Zstd: {:02.0}%]",
                    worker_id, lang, pct_db, pct_read, pct_proc, pct_zstd
                ));
            }

            if Some(num_articles_looked_at) == max_test_articles {
                break;
            }
        }

        metrics.dur_wall_time = zim_wall_clock.elapsed();
        let measured_sum =
            metrics.dur_db + metrics.dur_zim_read + metrics.dur_process + metrics.dur_compress;
        metrics.dur_overhead = metrics.dur_wall_time.saturating_sub(measured_sum);

        let mut local_zim_metrics = ZimMetrics {
            total_zim_files_processed: 1,
            ..Default::default()
        };
        local_zim_metrics
            .metrics_per_lang
            .insert(lang.clone(), metrics);

        pb.finish_with_message(format!("T{} | {} Done", worker_id, lang));
        tx.send(WorkItem::ZimFinished(path_string, local_zim_metrics))
            .unwrap();
    }
}

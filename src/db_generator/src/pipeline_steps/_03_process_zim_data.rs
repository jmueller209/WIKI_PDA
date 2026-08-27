use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
// use redb::{Database, TableDefinition};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

use crate::utils::article_processing;
use crate::utils::checkpoints;
use crate::utils::compression;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;
use crate::utils::txt_file_processing::{self, SortMode};

#[derive(Default, Debug)]
struct ZimMetrics {
    total_zim_files_processed: u64,
    articles_found_per_lang: HashMap<String, u64>,
    article_lookup_fails_per_lang: HashMap<String, u64>,
    total_db: std::time::Duration,
    total_zim_read: std::time::Duration,
    total_process: std::time::Duration,
    total_compress: std::time::Duration,
    total_overhead: std::time::Duration,
    total_worker_wall_time: std::time::Duration,

    pub program_total_duration: std::time::Duration,
    pub final_offset: u64,
    pub max_valid_offset: u64,
}

impl ZimMetrics {
    fn merge(&mut self, other: Self) {
        self.total_zim_files_processed += other.total_zim_files_processed;

        for (k, v) in other.articles_found_per_lang {
            *self.articles_found_per_lang.entry(k).or_insert(0) += v;
        }

        for (k, v) in other.article_lookup_fails_per_lang {
            *self.article_lookup_fails_per_lang.entry(k).or_insert(0) += v;
        }

        self.total_db += other.total_db;
        self.total_zim_read += other.total_zim_read;
        self.total_process += other.total_process;
        self.total_compress += other.total_compress;
        self.total_overhead += other.total_overhead;
        self.total_worker_wall_time += other.total_worker_wall_time;
    }

    pub fn make_summary(&self) -> String {
        let new_data_mb =
            (self.final_offset.saturating_sub(self.max_valid_offset)) as f64 / 1_048_576.0;
        let total_data_mb = self.final_offset as f64 / 1_048_576.0;

        let total_wall_secs = self.total_worker_wall_time.as_secs_f64().max(0.0001);

        let pct_db = (self.total_db.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_read = (self.total_zim_read.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_proc = (self.total_process.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_comp = (self.total_compress.as_secs_f64() / total_wall_secs) * 100.0;
        let pct_over = (self.total_overhead.as_secs_f64() / total_wall_secs) * 100.0;

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
            self.total_db,
            pct_db,
            self.total_zim_read,
            pct_read,
            self.total_process,
            pct_proc,
            self.total_compress,
            pct_comp,
            self.total_overhead,
            pct_over,
            self.total_worker_wall_time
        );

        let mut langs: Vec<&String> = self.articles_found_per_lang.keys().collect();
        langs.sort();

        for lang in langs {
            let found = self.articles_found_per_lang.get(lang).unwrap_or(&0);
            let fails = self
                .article_lookup_fails_per_lang
                .get(lang)
                .unwrap_or(&0);

            writeln!(&mut summary, "   - {:<18}", lang).unwrap();
            writeln!(&mut summary, "       Articles found:        {}", found).unwrap();
            writeln!(&mut summary, "       Article lookup fails:  {}", fails).unwrap();
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

    match checkpoints::checkpoint_exists(&settings, 3) {
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
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".into());
        }
        checkpoints::CheckpointState::DoesNotExist => (),
    }

    let program_start_time = Instant::now();

    let language_conf_path = &settings.paths.language_config_path;
    let languages_to_include: HashSet<String> = fs::read_to_string(language_conf_path)
        .expect("Failed to read language config")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let data_dir = PathBuf::from(&settings.paths.data_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let qid_idx_unsorted_txt_path =
        tmp_dir.join(constants::QID_INDEX_TXT.replace(".txt", "_unsorted.txt"));
    let qid_idx_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let zstd_dictionary_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);
    let content_bin_path = bin_dir.join(constants::CONTENT_BIN);
    // let sitelinks_qid_mapping_db_path = tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB);

    let text_delimiter = settings.other.text_delimiter.clone();
    let text_delim_str = text_delimiter.as_str();
    let ram_limit_mb = settings.performance.ram_limit_mb;

    let previously_completed_count = completed_zims.len() as u64;
    let mut zim_files_with_size: Vec<(PathBuf, u64, String)> = Vec::new(); // path, length,
                                                                           // language

    let dir = data_dir.join("wiki");
    let raw_pattern = &settings.match_patterns.wiki_zim_file_match_pattern;
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
                            zim_files_with_size.push((
                                path,
                                metadata.len(),
                                lang,
                            ));
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

    let mp_writer_clone = Arc::clone(&multi_progress);
    let content_bin_path_clone = content_bin_path.clone();
    let qid_idx_unsorted_path_clone = qid_idx_unsorted_txt_path.clone();

    let settings_writer_clone = settings.clone();

    let writer_thread = thread::spawn(move || {
        let content_bin_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&content_bin_path_clone)
            .unwrap();

        let qid_idx_unsorted_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&qid_idx_unsorted_path_clone)
            .unwrap();

        let mut bin_writer = BufWriter::new(content_bin_file);
        let mut idx_writer = BufWriter::new(qid_idx_unsorted_file);

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

                    let d = &settings_writer_clone.other.text_delimiter;
                    bin_writer.write_all(&article.binary_data).unwrap();
                    writeln!(
                        idx_writer,
                        "{0}{1}{2}{1}{3}{1}{4}{1}{5}",
                        article.qid, d, article.lang, current_offset, data_len, article.title
                    )
                    .unwrap();

                    current_offset += data_len;
                }
                WorkItem::ZimFinished(zim_path, local_metrics) => {
                    bin_writer.flush().unwrap();
                    idx_writer.flush().unwrap();

                    checkpoint_data.push_str(&zim_path);
                    checkpoint_data.push('\n');

                    if let Err(e) = checkpoints::make_checkpoint(
                        &settings_writer_clone,
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

                    mp_writer_clone
                        .println(format!(
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

        (global_metrics, current_offset)
    });

    let thread_count = settings.performance.thread_count;
    let worker_thread_count = std::cmp::max(1, thread_count - 2);

    println!("Starting {worker_thread_count} worker threads...");
    println!("Opening Key-Value Store for multi-threaded access...");

    let shared_db = Arc::new(sitelinks_lookup::open_sitelinks_db(&settings));

    thread::scope(|s| {
        for worker_id in 0..worker_thread_count {
            let tx_clone = tx.clone();
            let queue_clone = Arc::clone(&shared_queue);
            let mp_clone = Arc::clone(&multi_progress);
            let db_clone = Arc::clone(&shared_db);

            let zstd_dictionary_bin_path_clone = zstd_dictionary_bin_path.clone();

            s.spawn(move || {
                let read_txn = db_clone.begin_read().expect("Could not begin read transaction");
                let table = read_txn.open_table(sitelinks_lookup::SITELINKS_TABLE).expect("Table not found");

                let encoder_dict = compression::load_zstd_encoder_dictionary(&zstd_dictionary_bin_path_clone, settings.performance.zstd_compression_level).expect("Failed to load zstd encoder dictionary");

                let mut search_key = String::with_capacity(256);

                loop {
                    let next_zim = {
                        let mut queue = queue_clone.lock().unwrap();
                        queue.pop()
                    };

                    let (zim_path, _, lang) = match next_zim {
                        Some(data) => data,
                        None => break,
                    };

                    let path_string = zim_path.to_string_lossy().to_string();

                    let zim_file = zim::Zim::new(&zim_path).expect("Could not open/parse ZIM file");

                    let pb = mp_clone.add(ProgressBar::new(zim_file.header.article_count as u64));
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
                            .unwrap()
                            .progress_chars("#>-"),
                    );
                    pb.set_message(format!("T{} | {} (KV-Store)", worker_id, lang));

                    let mut articles_found = 0;
                    let mut lookup_fails = 0;
                    let mut num_articles_processed = 0;

                    let zim_wall_clock = Instant::now();
                    let mut dur_db = std::time::Duration::ZERO;
                    let mut dur_zim_read = std::time::Duration::ZERO;
                    let mut dur_process = std::time::Duration::ZERO;
                    let mut dur_compress = std::time::Duration::ZERO;

                    for direntry_result in zim_file.iterate_by_urls() {
                        pb.inc(1);

                        let direntry = match direntry_result {
                            Ok(entry) => entry,
                            Err(_) => continue,
                        };

                        if !matches!(direntry.namespace, zim::Namespace::Articles | zim::Namespace::UserContent) {
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

                        dur_db += t_db_lookup.elapsed();

                        let qid = match qid_opt {
                            Some(q) => q,
                            None => {
                                lookup_fails += 1;
                                continue;
                            }
                        };

                        let t_read = Instant::now();
                        let content = match zim_file.entry_content(&direntry) {
                            Ok(Some(c)) => c,
                            _ => {
                                lookup_fails += 1;
                                continue;
                            }
                        };

                        let article_text = match content.with(|bytes| {
                            unsafe { std::str::from_utf8_unchecked(bytes).to_string() }
                        }) {
                            Ok(text) => text,
                            Err(_) => {
                                lookup_fails += 1;
                                continue;
                            }
                        };
                        dur_zim_read += t_read.elapsed();

                        let t_process = Instant::now();

                        let process_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            article_processing::process_wikipedia_article(
                                &qid,
                                &article_text,
                                &table,
                                &mut search_key,
                                settings,
                                &lang
                            )
                        }));

                        let raw_bin_data = match process_result {
                            Ok(data) => data,
                            Err(_) => {
                                lookup_fails += 1;
                                continue; 
                            }
                        };
                        dur_process += t_process.elapsed();

                        let t_compress = Instant::now();
                        let compressed_data = compression::compress_data_zstd(&raw_bin_data, &encoder_dict, settings.performance.zstd_window_size_kb)
                            .expect("Failed to compress article with zstd");
                        dur_compress += t_compress.elapsed();

                        tx_clone
                            .send(WorkItem::Article(ProcessedArticle {
                                qid,
                                lang: lang.clone(),
                                title: primary_title,
                                binary_data: compressed_data,
                            }))
                            .expect("Writer thread died, could not send article");

                        articles_found += 1;
                        num_articles_processed += 1;

                        if num_articles_processed % 1000 == 0 {
                            let dur_total_measured = dur_db + dur_zim_read + dur_process + dur_compress;
                            let total = dur_total_measured.as_secs_f64().max(0.0001);

                            let pct_db = (dur_db.as_secs_f64() / total) * 100.0;
                            let pct_read = (dur_zim_read.as_secs_f64() / total) * 100.0;
                            let pct_proc = (dur_process.as_secs_f64() / total) * 100.0;
                            let pct_zstd = (dur_compress.as_secs_f64() / total) * 100.0;

                            pb.set_message(format!(
                                "T{} | {} (KV-Store) [DB: {:02.0}% | Read: {:02.0}% | Proc: {:02.0}% | Zstd: {:02.0}%]",
                                worker_id, lang, pct_db, pct_read, pct_proc, pct_zstd
                            ));
                        }

                        if Some(num_articles_processed) == max_test_articles {
                            break;
                        }
                    }

                    let wall_elapsed = zim_wall_clock.elapsed();
                    let measured_sum = dur_db + dur_zim_read + dur_process + dur_compress;
                    let dur_overhead = wall_elapsed.saturating_sub(measured_sum);

                    let mut local_metrics = ZimMetrics {
                        total_zim_files_processed: 1,
                        total_db: dur_db,
                        total_zim_read: dur_zim_read,
                        total_process: dur_process,
                        total_compress: dur_compress,
                        total_overhead: dur_overhead,
                        total_worker_wall_time: wall_elapsed,
                        ..Default::default()
                    };

                    local_metrics.articles_found_per_lang.insert(lang.clone(), articles_found);
                    local_metrics.article_lookup_fails_per_lang.insert(lang.clone(), lookup_fails);

                    pb.finish_with_message(format!("T{} | {} Done", worker_id, lang));
                    tx_clone.send(WorkItem::ZimFinished(path_string, local_metrics)).unwrap();
                }
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

    logs::write_summary_to_log(&summary, &settings, true, constants::ZIM_PROCESSING_LOG)?;

    checkpoints::make_checkpoint(&settings, 3, "zim_processing", None).map_err(|e| {
        format!(
            "Finished zim processing, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

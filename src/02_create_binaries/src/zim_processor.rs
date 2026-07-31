use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use redb::{Database, TableDefinition};
use regex::Regex;
use shared::article_processing::process_article;
use shared::compression::{compress_data_zstd, load_zstd_encoder_dictionary};
use shared::constants;
use shared::load_config::Settings;
use shared::txt_file_processing::{SortMode, external_merge_sort};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

const MAX_TEST_ARTICLES: Option<usize> = None;

#[derive(Default, Debug)]
struct ZimMetrics {
    total_zim_files_processed: u64,
    articles_found_per_wiki: HashMap<String, u64>,
    article_lookup_fails_per_wiki: HashMap<String, u64>,
    total_setup: std::time::Duration,
    total_db: std::time::Duration,
    total_zim_read: std::time::Duration,
    total_process: std::time::Duration,
    total_compress: std::time::Duration,
    total_overhead: std::time::Duration,
    total_worker_wall_time: std::time::Duration,
}

impl ZimMetrics {
    fn merge(&mut self, other: Self) {
        self.total_zim_files_processed += other.total_zim_files_processed;

        for (k, v) in other.articles_found_per_wiki {
            *self.articles_found_per_wiki.entry(k).or_insert(0) += v;
        }

        for (k, v) in other.article_lookup_fails_per_wiki {
            *self.article_lookup_fails_per_wiki.entry(k).or_insert(0) += v;
        }

        self.total_setup += other.total_setup;
        self.total_db += other.total_db;
        self.total_zim_read += other.total_zim_read;
        self.total_process += other.total_process;
        self.total_compress += other.total_compress;
        self.total_overhead += other.total_overhead;
        self.total_worker_wall_time += other.total_worker_wall_time;
    }
}

enum WorkItem {
    Article(ProcessedArticle),
    ZimFinished(String, ZimMetrics),
}

struct ProcessedArticle {
    qid: String,
    wiki_lang: String,
    binary_data: Vec<u8>,
}

pub fn process_directories(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let program_start_time = Instant::now();

    let language_conf_path = &settings.paths.language_config_path;
    let languages_to_include: HashSet<String> = fs::read_to_string(language_conf_path)
        .expect("Failed to read language config")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut completed_zims = HashSet::new();

    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let log_dir = PathBuf::from(&settings.paths.log_dir);
    let data_dir = PathBuf::from(&settings.paths.data_dir);
    let cache_dir = PathBuf::from(&settings.paths.cache_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let prog_file_path = cache_dir.join(constants::ZIM_PROGRESSION_CACHE);
    let qid_idx_unsorted_txt_path =
        tmp_dir.join(constants::QID_INDEX_TXT.replace(".txt", "_unsorted.txt"));
    let qid_idx_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let zstd_dictionary_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);
    let content_bin_path = bin_dir.join(constants::CONTENT_BIN);
    let sitelinks_qid_mapping_db_path = tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB);

    let text_delimiter = settings.other.text_delimiter.clone();
    let text_delim_str = text_delimiter.as_str();
    let ram_limit_mb = settings.performance.ram_limit_mb;

    if let Ok(file) = File::open(&prog_file_path) {
        for line in BufReader::new(file).lines().flatten() {
            completed_zims.insert(line);
        }
    }

    let previously_completed_count = completed_zims.len() as u64;
    let mut zim_files_with_size: Vec<(PathBuf, u64, String, String)> = Vec::new();

    for wiki in &settings.database_content.wikis_to_include {
        let dir = data_dir.join(wiki);

        let raw_pattern = match wiki.as_str() {
            "wiki" => &settings.match_patterns.wiki_zim_file_match_pattern,
            "wiktionary" => &settings.match_patterns.wiktionary_zim_file_match_pattern,
            "wikiquote" => &settings.match_patterns.wikiquote_zim_file_match_pattern,
            "wikisource" => &settings.match_patterns.wikisource_zim_file_match_pattern,
            "wikivoyage" => &settings.match_patterns.wikivoyage_zim_file_match_pattern,
            "wikiversity" => &settings.match_patterns.wikiversity_zim_file_match_pattern,
            "wikibooks" => &settings.match_patterns.wikibooks_zim_file_match_pattern,
            _ => continue,
        };

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
                                    wiki.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    let total_zims_to_process = zim_files_with_size.len();
    zim_files_with_size.sort_by_key(|&(_, size, _, _)| size);
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
            if parts.len() == 4 {
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
    let prog_file_path_clone = prog_file_path.clone();

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

        let prog_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&prog_file_path_clone)
            .unwrap();

        let mut bin_writer = BufWriter::new(content_bin_file);
        let mut idx_writer = BufWriter::new(qid_idx_unsorted_file);
        let mut prog_writer = BufWriter::new(prog_file);

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

                    bin_writer.write_all(&article.binary_data).unwrap();
                    writeln!(
                        idx_writer,
                        "{}\t{}\t{}\t{}",
                        article.qid, article.wiki_lang, current_offset, data_len
                    )
                    .unwrap();

                    current_offset += data_len;
                }
                WorkItem::ZimFinished(zim_path, local_metrics) => {
                    bin_writer.flush().unwrap();
                    idx_writer.flush().unwrap();

                    writeln!(prog_writer, "{}", zim_path).unwrap();
                    prog_writer.flush().unwrap();

                    global_metrics.merge(local_metrics);
                    global_metrics.total_zim_files_processed += 1;
                    processed_zims_this_run += 1;

                    let percentage =
                        (processed_zims_this_run as f64 / total_zims_to_process as f64) * 100.0;

                    mp_writer_clone
                        .println(format!(
                            "✅ ZIM finished: {} | Progress: {}/{} ({:.1}%)",
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

    let shared_db = Arc::new(
        Database::open(&sitelinks_qid_mapping_db_path).expect("Failed to open sitelink database"),
    );
    const SITELINKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sitelinks");

    thread::scope(|s| {
        for worker_id in 0..worker_thread_count {
            let tx_clone = tx.clone();
            let queue_clone = Arc::clone(&shared_queue);
            let mp_clone = Arc::clone(&multi_progress);
            let db_clone = Arc::clone(&shared_db);

            let zstd_dictionary_bin_path_clone = zstd_dictionary_bin_path.clone();
            s.spawn(move || {
                let read_txn = db_clone.begin_read().expect("Could not begin read transaction");
                let table = read_txn.open_table(SITELINKS_TABLE).expect("Table not found");

                let encoder_dict = load_zstd_encoder_dictionary(&zstd_dictionary_bin_path_clone, settings.performance.zstd_compression_level).expect("Failed to load zstd encoder dictionary");

                let mut search_key = String::with_capacity(256);

                loop {
                    let next_zim = {
                        let mut queue = queue_clone.lock().unwrap();
                        queue.pop()
                    };

                    let (zim_path, _, lang, wiki_type) = match next_zim {
                        Some(data) => data,
                        None => break,
                    };

                    let path_string = zim_path.to_string_lossy().to_string();
                    let combined_lang = format!("{}_{}", wiki_type, lang);

                    let zim_file = zim::Zim::new(&zim_path).expect("Could not open/parse ZIM file");

                    let pb = mp_clone.add(ProgressBar::new(zim_file.header.article_count as u64));
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
                            .unwrap()
                            .progress_chars("#>-"),
                    );
                    pb.set_message(format!("T{} | {} (KV-Store)", worker_id, combined_lang));

                    let mut articles_found = 0;
                    let mut lookup_fails = 0;
                    let mut num_articles_processed = 0;

                    let zim_wall_clock = Instant::now();
                    let mut dur_setup = std::time::Duration::ZERO;
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

                        let t_setup = Instant::now();
                        let decoded_url;
                        let raw_title = if !direntry.title.is_empty() {
                            direntry.title.as_str()
                        } else {
                            decoded_url = urlencoding::decode(&direntry.url).unwrap_or(std::borrow::Cow::Borrowed(&direntry.url));
                            &decoded_url
                        };
                        let primary_title = if raw_title.contains('_') {
                            raw_title.replace('_', " ").trim().to_string()
                        } else {
                            raw_title.trim().to_string()
                        };
                        dur_setup += t_setup.elapsed();

                        let t_db = Instant::now();
                        let mut qid = String::new();
                        let mut found = false;

                        search_key.clear();
                        search_key.push_str(&lang);
                        search_key.push_str(text_delim_str);
                        search_key.push_str(&wiki_type);
                        search_key.push_str(text_delim_str);
                        search_key.push_str(&primary_title);

                        if let Ok(Some(q)) = table.get(search_key.as_str()) {
                            qid = q.value().to_string();
                            found = true;
                        }
                        dur_db += t_db.elapsed();

                        if !found {
                            let t_setup_fb = Instant::now();
                            let decoded_url_fb = urlencoding::decode(&direntry.url).unwrap_or(std::borrow::Cow::Borrowed(&direntry.url));
                            let mut fallback_title = if decoded_url_fb.contains('_') {
                                decoded_url_fb.replace('_', " ").trim().to_string()
                            } else {
                                decoded_url_fb.trim().to_string()
                            };

                            if let Some(first_char) = fallback_title.chars().next() {
                                if first_char.is_lowercase() {
                                    let mut chars = fallback_title.chars();
                                    if let Some(f) = chars.next() {
                                        fallback_title = f.to_uppercase().collect::<String>() + chars.as_str();
                                    }
                                }
                            }
                            dur_setup += t_setup_fb.elapsed();

                            let t_db_fb = Instant::now();
                            search_key.clear();
                            search_key.push_str(&lang);
                            search_key.push_str(text_delim_str);
                            search_key.push_str(&wiki_type);
                            search_key.push_str(text_delim_str);
                            search_key.push_str(&fallback_title);

                            if let Ok(Some(q)) = table.get(search_key.as_str()) {
                                qid = q.value().to_string();
                                found = true;
                            }
                            dur_db += t_db_fb.elapsed();
                        }

                        if !found {
                            lookup_fails += 1;
                            continue;
                        }

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
                        let raw_bin_data = process_article(&wiki_type, &qid, &article_text);
                        dur_process += t_process.elapsed();

                        let t_compress = Instant::now();
                        let compressed_data = compress_data_zstd(&raw_bin_data, &encoder_dict, settings.performance.zstd_window_size_kb)
                            .expect("Failed to compress article with zstd");
                        dur_compress += t_compress.elapsed();

                        tx_clone
                            .send(WorkItem::Article(ProcessedArticle {
                                qid,
                                wiki_lang: combined_lang.clone(),
                                binary_data: compressed_data,
                            }))
                            .expect("Writer thread died, could not send article");

                        articles_found += 1;
                        num_articles_processed += 1;

                        if num_articles_processed % 1000 == 0 {
                            let dur_total_measured = dur_setup + dur_db + dur_zim_read + dur_process + dur_compress;
                            let total = dur_total_measured.as_secs_f64().max(0.0001);

                            let pct_setup = (dur_setup.as_secs_f64() / total) * 100.0;
                            let pct_db = (dur_db.as_secs_f64() / total) * 100.0;
                            let pct_read = (dur_zim_read.as_secs_f64() / total) * 100.0;
                            let pct_proc = (dur_process.as_secs_f64() / total) * 100.0;
                            let pct_zstd = (dur_compress.as_secs_f64() / total) * 100.0;

                            pb.set_message(format!(
                                "T{} | {} (KV-Store) [Setup: {:02.0}% | DB: {:02.0}% | Read: {:02.0}% | Proc: {:02.0}% | Zstd: {:02.0}%]",
                                worker_id, combined_lang, pct_setup, pct_db, pct_read, pct_proc, pct_zstd
                            ));
                        }

                        if Some(num_articles_processed) == MAX_TEST_ARTICLES {
                            break;
                        }
                    }

                    let wall_elapsed = zim_wall_clock.elapsed();
                    let measured_sum = dur_setup + dur_db + dur_zim_read + dur_process + dur_compress;
                    let dur_overhead = wall_elapsed.saturating_sub(measured_sum);

                    let mut local_metrics = ZimMetrics {
                        total_zim_files_processed: 1,
                        total_setup: dur_setup,
                        total_db: dur_db,
                        total_zim_read: dur_zim_read,
                        total_process: dur_process,
                        total_compress: dur_compress,
                        total_overhead: dur_overhead,
                        total_worker_wall_time: wall_elapsed,
                        ..Default::default()
                    };

                    local_metrics.articles_found_per_wiki.insert(combined_lang.clone(), articles_found);
                    local_metrics.article_lookup_fails_per_wiki.insert(combined_lang.clone(), lookup_fails);

                    pb.finish_with_message(format!("T{} | {} Done", worker_id, combined_lang));
                    tx_clone.send(WorkItem::ZimFinished(path_string, local_metrics)).unwrap();
                }
            });
        }
    });

    drop(tx);

    let (global_metrics, final_offset) = writer_thread.join().expect("Writer thread crashed");

    external_merge_sort(
        qid_idx_unsorted_txt_path.to_str().unwrap(),
        qid_idx_txt_path.to_str().unwrap(),
        SortMode::XId,
        ram_limit_mb,
        thread_count,
        &text_delimiter,
    )
    .expect("Failed to sort QID Index");

    let mut summary = String::new();

    writeln!(
        &mut summary,
        "\n=================================================="
    )
    .unwrap();
    writeln!(&mut summary, "📊 PROCESSING SUMMARY").unwrap();
    writeln!(
        &mut summary,
        "=================================================="
    )
    .unwrap();
    writeln!(
        &mut summary,
        "⏱️  Total duration:                {:.2?}",
        program_start_time.elapsed()
    )
    .unwrap();
    writeln!(
        &mut summary,
        "📦 Total ZIM files processed:     {} (Lifetime)",
        global_metrics.total_zim_files_processed
    )
    .unwrap();
    writeln!(
        &mut summary,
        "💾 Binary data written (New):     {:.2} MB",
        (final_offset - max_valid_offset) as f64 / 1_048_576.0
    )
    .unwrap();
    writeln!(
        &mut summary,
        "💾 Binary data total size:        {:.2} MB",
        final_offset as f64 / 1_048_576.0
    )
    .unwrap();
    writeln!(&mut summary, "\n📈 Breakdown by Wiki:").unwrap();

    let mut wikis: Vec<&String> = global_metrics.articles_found_per_wiki.keys().collect();
    wikis.sort();

    for wiki_lang in wikis {
        let found = global_metrics
            .articles_found_per_wiki
            .get(wiki_lang)
            .unwrap_or(&0);
        let fails = global_metrics
            .article_lookup_fails_per_wiki
            .get(wiki_lang)
            .unwrap_or(&0);

        writeln!(&mut summary, "   - {:<18}", wiki_lang).unwrap();
        writeln!(&mut summary, "       Articles found:        {}", found).unwrap();
        writeln!(&mut summary, "       Article lookup fails:  {}", fails).unwrap();
    }

    writeln!(
        &mut summary,
        "==================================================\n"
    )
    .unwrap();
    print!("{}", summary);

    let summary_log_path = Path::new(&log_dir).join(constants::BINARIES_LOG);

    if let Some(parent) = summary_log_path.parent() {
        fs::create_dir_all(parent).expect("Could not create logs directory!");
    }

    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&summary_log_path)
        .expect("Failed to open summary log file");

    log_file
        .write_all(summary.as_bytes())
        .expect("Failed to write summary to log file");

    Ok(())
}

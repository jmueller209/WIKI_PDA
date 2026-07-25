use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use regex::Regex;
use rusqlite::{Connection, params};
use shared::article_processing::process_article;
use shared::load_config::Settings;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Instant;

#[derive(Default, Debug)]
struct ZimMetrics {
    total_zim_files_processed: u64,
    articles_found_per_wiki: HashMap<String, u64>,
    article_lookup_fails_per_wiki: HashMap<String, u64>,
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
    let language_conf_path = &settings.paths.language_config_path;
    let languages_to_include: HashSet<String> = fs::read_to_string(language_conf_path)
        .expect("Failed to read language config")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let data_dir = Path::new(&settings.paths.data_dir);

    let mut completed_zims = HashSet::new();
    let prog_file_path = &settings.paths.progression_log_file_path;
    if let Ok(file) = File::open(prog_file_path) {
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

    let bin_file_path = settings.paths.content_bin_file_path.clone();
    let idx_file_path = settings
        .paths
        .qid_index_txt_file_path
        .replace(".txt", "_unsorted.txt");
    let prog_log_path = prog_file_path.clone();

    for path_str in [&bin_file_path, &idx_file_path, &prog_log_path] {
        if let Some(parent) = Path::new(path_str).parent() {
            fs::create_dir_all(parent).expect("Could not create missing directories!");
        }
    }

    let mut max_valid_offset: u64 = 0;
    if let Ok(idx_file) = File::open(&idx_file_path) {
        let reader = BufReader::new(idx_file);
        for line in reader.lines().flatten() {
            let parts: Vec<&str> = line.split('\t').collect();
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

    if let Ok(bin_file) = OpenOptions::new().write(true).open(&bin_file_path) {
        bin_file
            .set_len(max_valid_offset)
            .expect("Error repairing the .bin file");
    }

    let mp_writer_clone = Arc::clone(&multi_progress);

    let writer_thread = thread::spawn(move || {
        let bin_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&bin_file_path)
            .unwrap();
        let idx_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&idx_file_path)
            .unwrap();
        let prog_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&prog_log_path)
            .unwrap();

        let mut bin_writer = BufWriter::new(bin_file);
        let mut idx_writer = BufWriter::new(idx_file);
        let mut prog_writer = BufWriter::new(prog_file);

        let mut current_offset = max_valid_offset;
        let start_time = Instant::now();
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

        // Print final summary
        println!("\n==================================================");
        println!("📊 PROCESSING SUMMARY");
        println!("==================================================");
        println!(
            "⏱️  Total duration:               {:.2?}",
            start_time.elapsed()
        );
        println!(
            "📦 Total ZIM files processed:    {} (Lifetime)",
            global_metrics.total_zim_files_processed
        );
        println!(
            "💾 Binary data written (New):    {:.2} MB",
            (current_offset - max_valid_offset) as f64 / 1_048_576.0
        );
        println!(
            "💾 Binary data total size:       {:.2} MB",
            current_offset as f64 / 1_048_576.0
        );
        println!("\n📈 Breakdown by Wiki:");

        // Get unique wikis sorted
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

            println!("   - {:<18}", wiki_lang);
            println!("       Articles found:        {}", found);
            println!("       Article lookup fails:  {}", fails);
        }
        println!("==================================================\n");
    });

    let thread_count = settings.performance.thread_count;

    thread::scope(|s| {
        for worker_id in 0..thread_count {
            let tx_clone = tx.clone();
            let queue_clone = Arc::clone(&shared_queue);
            let mp_clone = Arc::clone(&multi_progress);

            let db_path = settings
                .paths
                .sitelinks_qid_mapping_txt_file_path
                .replace(".txt", ".sqlite");

            s.spawn(move || {
                let conn = Connection::open_with_flags(
                    &db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                        | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
                .expect("Worker could not open SQLite database!");

                let mut stmt = conn
                    .prepare("SELECT qid FROM sitelinks WHERE lang = ?1 AND wiki = ?2 AND title = ?3")
                    .expect("Could not prepare SQLite statement!");

                loop {
                    let next_zim = {
                        let mut queue = queue_clone.lock().unwrap();
                        queue.pop()
                    };

                    match next_zim {
                        Some((zim_path, _, lang, wiki_type)) => {
                            let path_string = zim_path.to_string_lossy().to_string();
                            let combined_lang = format!("{}_{}", wiki_type, lang);

                            let zim_file = zim::Zim::new(&zim_path).expect("Could not open/parse ZIM file");
                            let total_entries = zim_file.header.article_count as u64;
                            let mut local_metrics = ZimMetrics::default();

                            // Create a progress bar for this specific thread/file
                            let pb = mp_clone.add(ProgressBar::new(total_entries));
                            pb.set_style(ProgressStyle::default_bar()
                                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta}) {msg}")
                                .unwrap()
                                .progress_chars("#>-"));
                            pb.set_message(format!("T{} | {}", worker_id, combined_lang));

                            for direntry_result in zim_file.iterate_by_urls() {
                                pb.inc(1); // Advance progress bar

                                let direntry = match direntry_result {
                                    Ok(entry) => entry,
                                    Err(_) => continue,
                                };

                                match direntry.namespace {
                                    zim::Namespace::Articles | zim::Namespace::UserContent => {}
                                    _ => continue,
                                }

                                let title = direntry.url.replace('_', " ");
                                let qid_result: Result<String, _> = stmt.query_row(params![&lang, &wiki_type, &title], |row| row.get(0));

                                match qid_result {
                                    Ok(qid) => {
                                        if let Ok(Some(content)) = zim_file.entry_content(&direntry) {
                                            if let Ok(article_text) = content.with(|bytes| String::from_utf8_lossy(bytes).into_owned()) {
                                                let bin_data = process_article(&wiki_type, &qid, &article_text);
                                                tx_clone.send(WorkItem::Article(ProcessedArticle {
                                                    qid,
                                                    wiki_lang: combined_lang.clone(),
                                                    binary_data: bin_data,
                                                })).unwrap();

                                                *local_metrics.articles_found_per_wiki.entry(combined_lang.clone()).or_insert(0) += 1;
                                            } else {
                                                *local_metrics.article_lookup_fails_per_wiki.entry(combined_lang.clone()).or_insert(0) += 1;
                                            }
                                        } else {
                                            *local_metrics.article_lookup_fails_per_wiki.entry(combined_lang.clone()).or_insert(0) += 1;
                                        }
                                    }
                                    Err(_) => {
                                        *local_metrics.article_lookup_fails_per_wiki.entry(combined_lang.clone()).or_insert(0) += 1;
                                    }
                                }
                            }
                            pb.finish_with_message(format!("T{} | {} Done", worker_id, combined_lang));
                            tx_clone.send(WorkItem::ZimFinished(path_string, local_metrics)).unwrap();
                        }
                        None => break,
                    }
                }
            });
        }
    });

    drop(tx);
    writer_thread.join().expect("Writer thread crashed");

    Ok(())
}

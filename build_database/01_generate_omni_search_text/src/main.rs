use flate2::read::MultiGzDecoder;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

const CHUNK_SIZE: usize = 10_000;

#[derive(Deserialize, Debug)]
struct Sitelink {
    title: String,
}

#[derive(Deserialize, Debug)]
struct WikidataEntity {
    id: String,
    labels: Option<Value>,
    aliases: Option<Value>,
    sitelinks: Option<HashMap<String, Sitelink>>,
}

#[derive(Deserialize)]
struct MinimalEntity {
    labels: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Default, Debug)]
struct LangStats {
    labels: usize,
    aliases: usize,
    titles: usize,
}

fn find_repo_root() -> io::Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    let mut path = exe_path.ancestors();
    let repo_root = path.nth(5).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Failed to resolve repository root layout",
        )
    })?;
    Ok(repo_root.to_path_buf())
}

fn parse_config_file<P: AsRef<Path>>(path: P) -> HashMap<String, String> {
    let mut config_map = HashMap::new();
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.strip_prefix('#').unwrap_or(&line).trim();
            if !trimmed.is_empty() && trimmed.contains('=') {
                if let Some((key, val)) = trimmed.split_once('=') {
                    config_map.insert(key.trim().to_string(), val.trim().to_string());
                }
            }
        }
    }
    config_map
}

fn load_whitelist<P: AsRef<Path>>(path: P) -> io::Result<HashSet<u32>> {
    println!(
        "Loading whitelist into memory from {}...",
        path.as_ref().display()
    );
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut whitelist = HashSet::new();

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Ok(q_num) = trimmed.parse::<u32>() {
                whitelist.insert(q_num);
            }
        }
    }
    println!("Loaded {} approved entries into hashset.", whitelist.len());
    Ok(whitelist)
}

fn process_line(
    line: &str,
    whitelist: &HashSet<u32>,
) -> (Vec<String>, HashMap<String, LangStats>, HashSet<String>) {
    let mut extracted_entries = Vec::new();
    let mut local_stats: HashMap<String, LangStats> = HashMap::new();
    let mut local_skipped_langs: HashSet<String> = HashSet::new(); // NEW

    let mut whitelisted = false;
    if let Some(id_pos) = line.find("\"id\":\"Q") {
        let start = id_pos + 7;
        if let Some(end) = line[start..].find('"') {
            let q_str = &line[start..start + end];
            if let Ok(q_num) = q_str.parse::<u32>() {
                if whitelist.contains(&q_num) {
                    whitelisted = true;
                }
            }
        }
    }

    if !whitelisted {
        if let Ok(minimal) = serde_json::from_str::<MinimalEntity>(line) {
            if let Some(labels) = minimal.labels {
                for lang in labels.keys() {
                    local_skipped_langs.insert(lang.clone());
                }
            }
        }
        return (extracted_entries, local_stats, local_skipped_langs);
    }

    if let Ok(entity) = serde_json::from_str::<WikidataEntity>(line) {
        let q_id = &entity.id;

        if let Some(labels) = &entity.labels {
            if let Some(obj) = labels.as_object() {
                for (lang, val_obj) in obj {
                    if let Some(val) = val_obj.get("value").and_then(|v| v.as_str()) {
                        local_stats.entry(lang.clone()).or_default().labels += 1;
                        extracted_entries.push(format!("{}\t{}\tlabel\t{}\n", val, q_id, lang));
                    }
                }
            }
        }

        if let Some(aliases) = &entity.aliases {
            if let Some(obj) = aliases.as_object() {
                for (lang, alias_array_val) in obj {
                    if let Some(alias_array) = alias_array_val.as_array() {
                        for alias_obj in alias_array {
                            if let Some(val) = alias_obj.get("value").and_then(|v| v.as_str()) {
                                local_stats.entry(lang.clone()).or_default().aliases += 1;
                                extracted_entries
                                    .push(format!("{}\t{}\talias\t{}\n", val, q_id, lang));
                            }
                        }
                    }
                }
            }
        }

        if let Some(sitelinks) = &entity.sitelinks {
            for (wiki_key, sitelink) in sitelinks {
                if wiki_key.ends_with("wiki") {
                    let lang = wiki_key.strip_suffix("wiki").unwrap().to_string();
                    local_stats.entry(lang.clone()).or_default().titles += 1;
                    extracted_entries
                        .push(format!("{}\t{}\ttitle\t{}\n", sitelink.title, q_id, lang));
                }
            }
        }
    }

    (extracted_entries, local_stats, local_skipped_langs)
}
fn main() -> io::Result<()> {
    let start_time = Instant::now();
    let base_dir = find_repo_root()?;

    let args: Vec<String> = std::env::args().collect();
    let mut max_lines_limit: Option<usize> = None;
    if let Some(pos) = args.iter().position(|a| a == "--test-limit") {
        if let Some(val_str) = args.get(pos + 1) {
            if let Ok(val) = val_str.parse::<usize>() {
                max_lines_limit = Some(val);
                println!(
                    "🧪 TEST MODE ENABLED: Halting extraction after {} Q-IDs.",
                    val
                );
            }
        }
    }

    let config_path = base_dir.join("config").join("downloader.config");
    let processed_dir = base_dir.join("processed_files");
    let log_dir = base_dir.join("logs");

    let config_params = parse_config_file(&config_path);
    let configured_download_folder = config_params
        .get("DOWNLOAD_PATH")
        .map(String::as_str)
        .unwrap_or("zim_files");

    let json_gz_path = base_dir
        .join(configured_download_folder)
        .join("latest-all.json.gz");

    let whitelist_path = processed_dir.join("whitelist.txt");
    let output_path = processed_dir.join("omni_search_extracted.txt");
    let log_path = log_dir.join("omni_extraction_stats.log");

    let auto_config_path = base_dir.join("config").join("discovered_languages.config");

    let whitelist = load_whitelist(&whitelist_path)?;

    let mut global_stats: HashMap<String, LangStats> = HashMap::new();
    let mut global_skipped_langs: HashSet<String> = HashSet::new();

    println!(
        "Opening data dump stream archive: {}",
        json_gz_path.display()
    );
    let compressed_file = File::open(&json_gz_path)?;
    let gz_decoder = MultiGzDecoder::new(compressed_file);
    let reader = BufReader::with_capacity(256 * 1024, gz_decoder);

    let out_file = File::create(output_path)?;
    let mut writer = BufWriter::new(out_file);

    println!(
        "Spinning up multi-core processor matrix (Chunk size: {})...",
        CHUNK_SIZE
    );
    let mut processed_lines = 0;
    let mut entries_written = 0;
    let mut line_chunk = Vec::with_capacity(CHUNK_SIZE);

    for line_result in reader.lines() {
        let mut line = match line_result {
            Ok(l) => l,
            Err(e) => {
                println!("🚨 RUST STREAM ERROR AT LINE {}: {}", processed_lines, e);
                break;
            }
        };

        if line.trim() == "[" || line.trim() == "]" {
            continue;
        }
        if line.ends_with(',') {
            line.pop();
        }

        line_chunk.push(line);
        processed_lines += 1;

        if let Some(limit) = max_lines_limit {
            if processed_lines >= limit {
                println!("Test limit of {} reached. Halting stream...", limit);
                break;
            }
        }

        if processed_lines % 1_000_000 == 0 {
            let elapsed = start_time.elapsed().as_secs_f32();
            let rate = processed_lines as f32 / elapsed;
            println!(
                "⚡ Processed {} million lines... [{:.0} lines/sec]",
                processed_lines / 1_000_000,
                rate
            );
        }

        if line_chunk.len() >= CHUNK_SIZE {
            let chunk_results: Vec<(Vec<String>, HashMap<String, LangStats>, HashSet<String>)> =
                line_chunk
                    .par_iter()
                    .map(|raw_line| process_line(raw_line, &whitelist))
                    .collect();

            for (entries, local_map, local_skipped) in chunk_results {
                for entry in entries {
                    writer.write_all(entry.as_bytes())?;
                    entries_written += 1;
                }
                for (lang, stats) in local_map {
                    let global_lang_stats = global_stats.entry(lang).or_default();
                    global_lang_stats.labels += stats.labels;
                    global_lang_stats.aliases += stats.aliases;
                    global_lang_stats.titles += stats.titles;
                }
                for lang in local_skipped {
                    global_skipped_langs.insert(lang);
                }
            }
            line_chunk.clear();
        }
    }

    if !line_chunk.is_empty() {
        let chunk_results: Vec<(Vec<String>, HashMap<String, LangStats>, HashSet<String>)> =
            line_chunk
                .par_iter()
                .map(|raw_line| process_line(raw_line, &whitelist))
                .collect();

        for (entries, local_map, local_skipped) in chunk_results {
            for entry in entries {
                writer.write_all(entry.as_bytes())?;
                entries_written += 1;
            }
            for (lang, stats) in local_map {
                let global_lang_stats = global_stats.entry(lang).or_default();
                global_lang_stats.labels += stats.labels;
                global_lang_stats.aliases += stats.aliases;
                global_lang_stats.titles += stats.titles;
            }
            for lang in local_skipped {
                global_skipped_langs.insert(lang);
            }
        }
    }

    writer.flush()?;

    println!("Writing metrics breakdown to {}...", log_path.display());
    let mut log_file = File::create(log_path)?;

    let mut sorted_stats: Vec<(&String, &LangStats)> = global_stats.iter().collect();
    sorted_stats.sort_by(|a, b| {
        let total_b = b.1.labels + b.1.aliases + b.1.titles;
        let total_a = a.1.labels + a.1.aliases + a.1.titles;
        total_b.cmp(&total_a)
    });

    for (lang, stats) in &sorted_stats {
        writeln!(log_file, "=== Language Matrix Profile: {} ===", lang)?;
        writeln!(log_file, "  labels:  {}", stats.labels)?;
        writeln!(log_file, "  aliases: {}", stats.aliases)?;
        writeln!(log_file, "  titles:  {}", stats.titles)?;
        writeln!(
            log_file,
            "  total:   {}",
            stats.labels + stats.aliases + stats.titles
        )?;
        writeln!(log_file)?;
    }

    println!(
        "Generating discoverable language configurations at {}...",
        auto_config_path.display()
    );
    let mut config_file = File::create(auto_config_path)?;

    let mut alphabetical_langs: Vec<&String> = global_stats.keys().collect();
    alphabetical_langs.sort();

    for lang in alphabetical_langs {
        writeln!(config_file, "{}", lang)?;
    }

    println!("\n--- EXTRACTION COMPLETE ---");
    println!("Processed total lines: {}", processed_lines);
    println!("Total flat entries written: {}", entries_written);
    println!("Execution time: {:?}", start_time.elapsed());
    println!(
        "Total distinct database dialects cataloged: {}",
        global_stats.len()
    );

    println!("\n--- ORPHAN LANGUAGE ANALYSIS ---");

    let discovered_langs: HashSet<String> = global_stats.keys().cloned().collect();

    let mut orphans: Vec<&String> = global_skipped_langs.difference(&discovered_langs).collect();
    orphans.sort();

    if orphans.is_empty() {
        println!("✅ No orphan languages found! All database dialects are represented in notable entities.");
    } else {
        println!("👻 Found {} 'Orphan Languages' (used in raw data, but never on notable Wikipedia entities):", orphans.len());

        for (i, lang) in orphans.iter().enumerate() {
            if i < 30 {
                print!("{}, ", lang);
            }
        }
        if orphans.len() > 30 {
            print!("...and {} more.", orphans.len() - 30);
        }
        println!();

        writeln!(log_file, "\n=========================================")?;
        writeln!(log_file, "=== ORPHAN LANGUAGES (SKIPPED ITEMS)  ===")?;
        writeln!(log_file, "=========================================")?;
        for lang in &orphans {
            writeln!(log_file, "{}", lang)?;
        }
    }

    Ok(())
}

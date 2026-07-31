use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use regex::Regex;
use shared::article_processing::process_article;
use shared::compression::train_and_save_zstd_dictionary;
use shared::constants;
use shared::load_config::Settings;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn generate_zstd_dictionary(settings: &Settings) {
    let wikis_to_include = &settings.database_content.wikis_to_include;
    let language_conf_path = &settings.paths.language_config_path;
    let languages_to_include: HashSet<String> = fs::read_to_string(language_conf_path)
        .expect("Failed to read language config")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let data_dir = Path::new(&settings.paths.data_dir);
    let bin_dir = Path::new(&settings.paths.bin_dir);
    let zstd_dictionary_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);

    let zstd_dict_size_kb = settings.performance.zstd_dict_size_kb;
    let zstd_sample_size_mb = settings.performance.zstd_dict_training_sample_size_mb;

    if Path::new(&zstd_dictionary_bin_path).exists() {
        println!(
            "Dictionary already exists at {:?}. Skipping generation.",
            zstd_dictionary_bin_path
        );
        return;
    }

    let mut allowed_zim_files_with_size: Vec<(PathBuf, u64, String)> = Vec::new();

    for wiki in wikis_to_include {
        let dir = data_dir.join(wiki);

        let raw_pattern = match wiki.as_str() {
            "wiki" => &settings.match_patterns.wiki_zim_file_match_pattern,
            "wiktionary" => &settings.match_patterns.wiktionary_zim_file_match_pattern,
            "wikiquote" => &settings.match_patterns.wikiquote_zim_file_match_pattern,
            "wikisource" => &settings.match_patterns.wikisource_zim_file_match_pattern,
            "wikivoyage" => &settings.match_patterns.wikivoyage_zim_file_match_pattern,
            "wikiversity" => &settings.match_patterns.wikiversity_zim_file_match_pattern,
            "wikibooks" => &settings.match_patterns.wikibooks_zim_file_match_pattern,
            _ => {
                eprintln!(
                    "Warning: No match pattern defined for wiki '{}'. Skipping.",
                    wiki
                );
                continue;
            }
        };

        let regex_str = raw_pattern.replace("{lang}", "(?P<lang>[a-zA-Z-]+)");
        let re = Regex::new(&regex_str).expect("Invalid Regex Pattern in config");

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                if let Some(captures) = re.captures(&filename) {
                    if let Some(lang_match) = captures.name("lang") {
                        let lang = lang_match.as_str();
                        if languages_to_include.contains(lang) {
                            if let Ok(metadata) = fs::metadata(&path) {
                                allowed_zim_files_with_size.push((
                                    path,
                                    metadata.len(),
                                    wiki.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if allowed_zim_files_with_size.is_empty() {
        eprintln!("Warning: No valid ZIM files found for dictionary training.");
        return;
    }

    let target_sample_bytes = zstd_sample_size_mb * 1024 * 1024;
    let mut current_sample_bytes = 0;

    let total_size: u64 = allowed_zim_files_with_size
        .iter()
        .map(|(_, size, _)| *size)
        .sum();
    let mut rng = rand::thread_rng();

    println!(
        "Gathering dictionary samples. Target size: {} MB",
        zstd_sample_size_mb
    );
    let pb = ProgressBar::new(target_sample_bytes as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/blue} {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Sampling articles...");

    let mut samples: Vec<Vec<u8>> = Vec::new();

    let mut open_zims: HashMap<PathBuf, zim::Zim> = HashMap::new();

    while current_sample_bytes < target_sample_bytes {
        let mut random_weight = rng.gen_range(0..total_size);
        let mut selected_zim_path = &allowed_zim_files_with_size[0].0;
        let mut selected_wiki_type = &allowed_zim_files_with_size[0].2;

        for (path, size, wiki_type) in &allowed_zim_files_with_size {
            if random_weight < *size {
                selected_zim_path = path;
                selected_wiki_type = wiki_type;
                break;
            }
            random_weight = random_weight.saturating_sub(*size);
        }

        let zim_file = open_zims
            .entry(selected_zim_path.clone())
            .or_insert_with(|| {
                zim::Zim::new(selected_zim_path).expect("Could not open ZIM file for training")
            });

        let total_entries = zim_file.header.article_count;
        if total_entries == 0 {
            continue;
        }

        let random_index = rng.gen_range(0..total_entries) as u32;

        if let Ok(direntry) = zim_file.get_by_url_index(random_index) {
            match direntry.namespace {
                zim::Namespace::Articles | zim::Namespace::UserContent => {}
                _ => continue,
            }
            if let Ok(Some(content)) = zim_file.entry_content(&direntry) {
                if let Ok(article_text) =
                    content.with(|bytes| String::from_utf8_lossy(bytes).into_owned())
                {
                    let bin_data = process_article(selected_wiki_type, "Q_TRAIN", &article_text);

                    if !bin_data.is_empty() {
                        let len = bin_data.len();
                        current_sample_bytes += len;
                        samples.push(bin_data);
                        pb.inc(len as u64);
                    }
                }
            }
        }
    }

    pb.finish_with_message("Done sampling.");

    train_and_save_zstd_dictionary(
        &samples,
        zstd_dict_size_kb * 1024,
        &zstd_dictionary_bin_path,
    )
    .expect("Critical Pipeline Error: Failed to generate Zstd dictionary");
}

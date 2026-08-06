use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::article_processing;
use crate::utils::checkpoints;
use crate::utils::compression;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;

pub struct CompressionMetrics {
    pub zstd_compression_level: i8,
    pub target_sample_size_mb: usize,
    pub train_sample_size_bytes: usize,
    pub train_articles_sampled: usize,
    pub test_sample_size_bytes: usize,
    pub test_articles_sampled: usize,
    pub num_zim_files_available: usize,
    pub target_dictionary_size_bytes: usize,
    pub compression_test_report: String,
}

impl CompressionMetrics {
    pub fn make_summary(&self) -> String {
        let train_mb = self.train_sample_size_bytes as f64 / (1024.0 * 1024.0);
        let test_mb = self.test_sample_size_bytes as f64 / (1024.0 * 1024.0);
        let dict_kb = self.target_dictionary_size_bytes as f64 / 1024.0;

        format!(
            "============================================================\n\
             =              DICTIONARY GENERATION SUMMARY               =\n\
             ============================================================\n\
             Source Material:\n\
             - Available ZIM files:      {}\n\
             - Target sample size:       {} MB per pass\n\
             \n\
             Training Phase:\n\
             - ZSTD compression level:   {}\n\
             - Actual data sampled:      {:.2} MB ({} bytes)\n\
             - Total articles sampled:   {}\n\
             \n\
             Testing Phase (Unbiased):\n\
             - Actual data sampled:      {:.2} MB ({} bytes)\n\
             - Total articles sampled:   {}\n\
             \n\
             Dictionary Details:\n\
             - Target dictionary size:   {:.2} KB ({} bytes)\n\
             \n\
             {}\n\
             ============================================================",
            self.num_zim_files_available,
            self.target_sample_size_mb,
            self.zstd_compression_level,
            train_mb,
            self.train_sample_size_bytes,
            self.train_articles_sampled,
            test_mb,
            self.test_sample_size_bytes,
            self.test_articles_sampled,
            dict_kb,
            self.target_dictionary_size_bytes,
            self.compression_test_report
        )
    }
}

pub fn generate_zstd_dictionary(settings: &Settings) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 2) {
        checkpoints::CheckpointState::exists_empty => {
            println!("Checkpoint found: Compression Setup has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::exists_with_data(data) => {
            return Err(format!(
                "Download checkpoint should not contain any data, but contains: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::exists_in_bad_state(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".to_string());
        }
        checkpoints::CheckpointState::does_not_exist => (),
    }

    let wikis_to_include = &settings.database_content.wikis_to_include;
    let language_conf_path = &settings.paths.language_config_path;

    let languages_to_include: HashSet<String> = fs::read_to_string(language_conf_path)
        .map_err(|e| {
            format!(
                "Failed to read language config at {:?}: {}",
                language_conf_path, e
            )
        })?
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let data_dir = Path::new(&settings.paths.data_dir);
    let bin_dir = Path::new(&settings.paths.bin_dir);
    let zstd_dictionary_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);

    let zstd_dict_size_kb = settings.performance.zstd_dict_size_kb;
    let zstd_sample_size_mb = settings.performance.zstd_dict_training_sample_size_mb;
    let zstd_compression_level = settings.performance.zstd_compression_level;
    let zstd_window_size_kb = settings.performance.zstd_window_size_kb;

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
        let re = Regex::new(&regex_str)
            .map_err(|e| format!("Invalid Regex Pattern '{}' in config: {}", regex_str, e))?;

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    let filename = file_name.to_string_lossy().to_string();
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
    }

    let num_zim_files_available = allowed_zim_files_with_size.len();
    if num_zim_files_available == 0 {
        return Err("No valid ZIM files found for dictionary training.".to_string());
    }

    let (train_samples, train_sample_size_bytes) = create_random_samples(
        &allowed_zim_files_with_size,
        zstd_sample_size_mb,
        "Sampling for training...",
    )?;

    let target_dictionary_size_bytes = zstd_dict_size_kb * 1024;
    compression::train_and_save_zstd_dictionary(
        &train_samples,
        target_dictionary_size_bytes,
        &zstd_dictionary_bin_path,
    )
    .map_err(|e| {
        format!(
            "Critical Pipeline Error: Failed to generate Zstd dictionary: {}",
            e
        )
    })?;

    let train_articles_count = train_samples.len();
    drop(train_samples);

    let (test_samples, test_sample_size_bytes) = create_random_samples(
        &allowed_zim_files_with_size,
        zstd_sample_size_mb,
        "Sampling for unbiased testing...",
    )?;

    println!("Testing newly created dictionary with fresh samples...");
    let report_string = compression::test_zstd_compression_rate(
        &test_samples,
        &zstd_dictionary_bin_path,
        zstd_compression_level,
        zstd_window_size_kb,
    )
    .map_err(|e| format!("Failed to test dictionary compression rate: {}", e))?;

    let metrics = CompressionMetrics {
        zstd_compression_level: zstd_compression_level as i8,
        target_sample_size_mb: zstd_sample_size_mb,
        train_sample_size_bytes,
        train_articles_sampled: train_articles_count,
        test_sample_size_bytes,
        test_articles_sampled: test_samples.len(),
        num_zim_files_available,
        target_dictionary_size_bytes,
        compression_test_report: report_string,
    };

    logs::write_summary_to_log(
        &metrics.make_summary(),
        &settings,
        true,
        constants::COMPRESSION_SETUP_LOG,
    )
    .map_err(|e| e.to_string())?;

    checkpoints::make_checkpoint(&settings, 2, "compression_setup", None).map_err(|e| {
        format!(
            "Finished compression setup, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

fn create_random_samples(
    allowed_zim_files_with_size: &[(PathBuf, u64, String)],
    target_sample_size_mb: usize,
    progress_msg: &str,
) -> Result<(Vec<Vec<u8>>, usize), String> {
    if allowed_zim_files_with_size.is_empty() {
        return Err("No valid ZIM files available for sampling.".to_string());
    }

    let target_sample_bytes = target_sample_size_mb * 1024 * 1024;
    let mut current_sample_bytes = 0;

    let total_size: u64 = allowed_zim_files_with_size
        .iter()
        .map(|(_, size, _)| *size)
        .sum();

    if total_size == 0 {
        return Err("Total size of valid ZIM files is 0 bytes.".to_string());
    }

    let mut rng = rand::thread_rng();

    println!(
        "Gathering samples ({}). Target size: {} MB",
        progress_msg, target_sample_size_mb
    );

    let pb = ProgressBar::new(target_sample_bytes as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.green/blue} {bytes}/{total_bytes} ({eta}) {msg}")
            .map_err(|e| format!("Failed to set progress bar template: {}", e))?
            .progress_chars("#>-"),
    );
    pb.set_message(progress_msg.to_string());

    let mut samples: Vec<Vec<u8>> = Vec::new();
    let mut open_zims: HashMap<PathBuf, zim::Zim> = HashMap::new();

    while current_sample_bytes < target_sample_bytes {
        let mut random_weight = rng.gen_range(0..total_size);
        let mut selected_zim_path = &allowed_zim_files_with_size[0].0;
        let mut selected_wiki_type = &allowed_zim_files_with_size[0].2;

        for (path, size, wiki_type) in allowed_zim_files_with_size {
            if random_weight < *size {
                selected_zim_path = path;
                selected_wiki_type = wiki_type;
                break;
            }
            random_weight = random_weight.saturating_sub(*size);
        }

        if !open_zims.contains_key(selected_zim_path) {
            let new_zim = zim::Zim::new(selected_zim_path).map_err(|e| {
                format!(
                    "Could not open ZIM file for training at {:?}: {}",
                    selected_zim_path, e
                )
            })?;
            open_zims.insert(selected_zim_path.clone(), new_zim);
        }

        let zim_file = open_zims.get(selected_zim_path).unwrap();

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
                    let bin_data = article_processing::process_article(
                        selected_wiki_type,
                        "Q_TRAIN",
                        &article_text,
                    );

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

    pb.finish_with_message(format!("Done {}.", progress_msg));
    Ok((samples, current_sample_bytes))
}

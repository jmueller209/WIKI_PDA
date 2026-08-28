use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::StatusCode;
use scraper::{Html, Selector};
use std::fmt::Write;
use std::fs;
use std::fs::OpenOptions;
use std::io::copy;
use std::path::Path;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;

struct DownloadMetrics {
    finished_downloads: Vec<String>,
    failed_retrievals: Vec<String>,
    failed_downloads: Vec<String>,
}

impl DownloadMetrics {
    fn new() -> Self {
        return DownloadMetrics {
            finished_downloads: Vec::new(),
            failed_retrievals: Vec::new(),
            failed_downloads: Vec::new(),
        };
    }

    fn merge(&mut self, other: DownloadMetrics) {
        self.finished_downloads.extend(other.finished_downloads);
        self.failed_retrievals.extend(other.failed_retrievals);
        self.failed_downloads.extend(other.failed_downloads);
    }
}

pub fn download_data(settings: &Settings) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 0) {
        checkpoints::CheckpointState::ExistsEmpty => {
            println!("Checkpoint found: Download has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::ExistsWithData(data) => {
            return Err(format!(
                "Download checkpoint should not contain any data, but contains: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::ExistsInBadState(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".to_string());
        }
        checkpoints::CheckpointState::DoesNotExist => (),
    }
    println!("Starting Download");

    let data_dir_path = &settings.paths.data_dir;

    let languages_config_path = &settings.paths.language_config_path;
    let languages = fs::read_to_string(languages_config_path)
        .map_err(|e| format!("Failed to read the language config: {e}"))?;

    let languages_vec: Vec<String> = languages
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    let client = reqwest::blocking::Client::builder()
        .user_agent("Offline Wikipedia Database")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let wiki_data_url = &settings.urls.wikidata_dump_url;
    println!("Downloading wikidata dump from: {}", wiki_data_url);
    download_wikidata_dump(wiki_data_url, &settings.paths.wikidata_dump_path, &client)?;

    let wikis_to_include = &settings.database_content.wikis_to_include;
    let mut total_metrics = DownloadMetrics::new();
    for wiki in wikis_to_include {
        match wiki.as_str() {
            "wiki" => {
                println!("Searching wikipedia...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wiki_base_url,
                    &settings.match_patterns.wiki_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path).join("wiki").to_str().unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wiktionary" => {
                println!("Searching wiktionary...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wiktionary_base_url,
                    &settings.match_patterns.wiktionary_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path)
                        .join("wiktionary")
                        .to_str()
                        .unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wikiquote" => {
                println!("Searching wikiquote...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wikiquote_base_url,
                    &settings.match_patterns.wikiquote_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path).join("wikiquote").to_str().unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wikisource" => {
                println!("Searching wikisource...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wikisource_base_url,
                    &settings.match_patterns.wikisource_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path)
                        .join("wikisource")
                        .to_str()
                        .unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wikivoyage" => {
                println!("Searching wikivoyage...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wikivoyage_base_url,
                    &settings.match_patterns.wikivoyage_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path)
                        .join("wikivoyage")
                        .to_str()
                        .unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wikiversity" => {
                println!("Searching wikiversity...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wikiversity_base_url,
                    &settings.match_patterns.wikiversity_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path)
                        .join("wikiversity")
                        .to_str()
                        .unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            "wikibooks" => {
                println!("Searching wikibooks...");
                let new_metrics = download_wikis_from_base_url(
                    &settings.urls.wikibooks_base_url,
                    &settings.match_patterns.wikibooks_zim_file_match_pattern,
                    &languages_vec,
                    &client,
                    &Path::new(data_dir_path).join("wikibooks").to_str().unwrap(),
                )?;
                total_metrics.merge(new_metrics);
            }
            _ => {
                return Err(
                    "Found invalid wiki in config 'wikis_to_include' while downloading data."
                        .to_string(),
                );
            }
        }
    }

    make_summary(total_metrics, &settings)?;

    checkpoints::make_checkpoint(&settings, 0, "downloads", None).map_err(|e| {
        format!(
            "Finished downloading, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

fn download_wikidata_dump(
    url: &str,
    download_path: &str,
    client: &reqwest::blocking::Client,
) -> Result<(), String> {
    let path = Path::new(download_path);

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Could not extract file name from path: {}", download_path))?;

    let download_dir = path.parent().and_then(|dir| dir.to_str()).unwrap_or(".");
    download_file(url, download_dir, file_name, client)?;

    Ok(())
}

fn download_wikis_from_base_url(
    base_url: &str,
    match_pattern: &str,
    languages: &Vec<String>,
    client: &reqwest::blocking::Client,
    download_dir: &str,
) -> Result<DownloadMetrics, String> {
    let response_text = client
        .get(base_url)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?
        .text()
        .map_err(|e| format!("Failed to read response text: {}", e))?;

    let document = Html::parse_document(&response_text);
    let selector = Selector::parse("a").unwrap();
    let mut files_to_download = Vec::new();

    let mut failed_retrievals: Vec<String> = Vec::new();

    for lang in languages {
        let pattern_string = match_pattern.replace("{lang}", lang);

        let re =
            Regex::new(&pattern_string).map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let mut latest_file: Option<String> = None;

        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if re.is_match(href) {
                    match &latest_file {
                        Some(current_latest) if href > current_latest.as_str() => {
                            latest_file = Some(href.to_string());
                        }
                        None => {
                            latest_file = Some(href.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(file) = latest_file {
            files_to_download.push(file);
        } else {
            failed_retrievals.push(pattern_string);
        }
    }

    let mut failed_downloads: Vec<String> = Vec::new();
    let mut finished_downloads: Vec<String> = Vec::new();
    for file in &files_to_download {
        let url = format!("{}/{}", base_url, file);
        println!("Starting Download from {:?}", url);
        match download_file(&url, download_dir, file, client) {
            Ok(_) => {
                finished_downloads.push(String::from(file));
            }
            Err(e) => {
                let failed_download_msg = format!("Failed downloading {} with error: {}", file, e);
                failed_downloads.push(failed_download_msg);
            }
        }
    }

    let metrics = DownloadMetrics {
        finished_downloads,
        failed_retrievals,
        failed_downloads,
    };
    Ok(metrics)
}

fn download_file(
    url: &str,
    download_dir: &str,
    file_name: &str,
    client: &reqwest::blocking::Client,
) -> Result<(), String> {
    std::fs::create_dir_all(download_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let destination_path = Path::new(download_dir).join(file_name);

    let head_response = client
        .head(url)
        .send()
        .map_err(|e| format!("HEAD request failed: {}", e))?;

    if !head_response.status().is_success() {
        return Err(format!(
            "Server returned error status on HEAD request: {}",
            head_response.status()
        ));
    }

    let mut total_size_opt = head_response.content_length();
    if total_size_opt == Some(0) {
        total_size_opt = None;
    }

    let mut local_size = 0;
    if destination_path.exists() {
        if let Ok(metadata) = fs::metadata(&destination_path) {
            local_size = metadata.len();
        }
    }

    if let Some(total_size) = total_size_opt {
        if local_size == total_size {
            println!("File already fully downloaded, skipping...");
            return Ok(());
        }
    }

    let mut request = client.get(url);

    if local_size > 0 {
        if let Some(total_size) = total_size_opt {
            if local_size < total_size {
                println!("Resuming download from byte {}...", local_size);
                request = request.header("Range", format!("bytes={}-", local_size));
            } else if local_size > total_size {
                println!("Local file is larger than remote file. Restarting download...");
                local_size = 0;
            }
        } else {
            println!(
                "Unknown remote file size. Attempting to resume from byte {}...",
                local_size
            );
            request = request.header("Range", format!("bytes={}-", local_size));
        }
    }

    let response = request
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
        println!("Server returned 416. The file is already fully downloaded, skipping...");
        return Ok(());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Server returned error status: {}",
            response.status()
        ));
    }

    let is_partial = response.status() == StatusCode::PARTIAL_CONTENT;
    if !is_partial && local_size > 0 {
        println!("Server does not support resuming or file changed. Restarting download...");
        local_size = 0;
    }

    let final_total_size = if let Some(total) = total_size_opt {
        total
    } else if let Some(rem_size) = response.content_length() {
        if is_partial {
            local_size + rem_size
        } else {
            rem_size
        }
    } else {
        0
    };

    let mut dest_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(is_partial)
        .truncate(!is_partial)
        .open(&destination_path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let pb = ProgressBar::new(final_total_size);

    let pb_template = if final_total_size > 0 {
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})"
    } else {
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes} downloaded (unknown total)"
    };

    pb.set_style(
        ProgressStyle::default_bar()
            .template(pb_template)
            .map_err(|e| format!("Failed to set progress bar style: {}", e))?
            .progress_chars("#>-"),
    );

    if local_size > 0 && is_partial {
        pb.set_position(local_size);
    }

    let mut source = pb.wrap_read(response);

    copy(&mut source, &mut dest_file)
        .map_err(|e| format!("Failed to write data to file: {}", e))?;

    Ok(())
}

fn make_summary(total_metrics: DownloadMetrics, settings: &Settings) -> Result<(), String> {
    let mut summary_string = String::new();
    let _ = writeln!(
        &mut summary_string,
        "\n================ DOWNLOAD SUMMARY ================"
    );
    let _ = writeln!(
        &mut summary_string,
        "Total Successful: {}",
        total_metrics.finished_downloads.len()
    );
    let _ = writeln!(
        &mut summary_string,
        "Total Failed: {}",
        total_metrics.failed_downloads.len()
    );
    let _ = writeln!(
        &mut summary_string,
        "Total Missing on Server: {}",
        total_metrics.failed_retrievals.len()
    );
    let _ = writeln!(&mut summary_string, "=============================\n");

    let _ = writeln!(&mut summary_string, "SUCCESSFULLY DOWNLOADED:");
    if total_metrics.finished_downloads.is_empty() {
        let _ = writeln!(&mut summary_string, "  None");
    } else {
        for file in &total_metrics.finished_downloads {
            let _ = writeln!(&mut summary_string, "  - {}", file);
        }
    }

    let _ = writeln!(&mut summary_string, "\nFAILED DOWNLOADS:");
    if total_metrics.failed_downloads.is_empty() {
        let _ = writeln!(&mut summary_string, "  None");
    } else {
        for error_msg in &total_metrics.failed_downloads {
            let _ = writeln!(&mut summary_string, "  - {}", error_msg);
        }
    }

    let _ = writeln!(
        &mut summary_string,
        "\nFAILED RETRIEVALS (No matches found for regex):"
    );
    if total_metrics.failed_retrievals.is_empty() {
        let _ = writeln!(&mut summary_string, "  None");
    } else {
        for pattern in &total_metrics.failed_retrievals {
            let _ = writeln!(&mut summary_string, "  - {}", pattern);
        }
    }
    writeln!(
        &mut summary_string,
        "========================================================"
    )
    .unwrap();

    logs::write_summary_to_log(&summary_string, &settings, true, constants::DOWNLOAD_LOG)?;

    Ok(())
}

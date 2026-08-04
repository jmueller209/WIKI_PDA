use crate::utils::settings::Settings;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::copy;
use std::path::Path;

pub fn download_data(settings: &Settings) -> Result<(), String> {
    // get relevant download path
    let data_dir_path = &settings.paths.data_dir;
    let cache_dir_path = &settings.paths.cache_dir;

    // download wikidata dump
    download_wikida_dump(&settings.urls.wikidata_dump_url, data_dir_path)?;

    // get relevant wikis
    let wikis_to_include = &settings.database_content.wikis_to_include;
    for wiki in wikis_to_include {
        println!("wiki: {wiki:?}");
        match wiki.as_str() {
            "wiki" => {
                println!("found wiki");
                download_wiki_from_url(
                    &settings.urls.wiki_base_url,
                    &settings.match_patterns.wiki_zim_file_match_pattern,
                )?;
            }
            "wiktionary" => {
                println!("unknown")
            }
            "wikiquote" => {}
            "wikisource" => {}
            "wikivoyage" => {}
            "wikinews" => {}
            "wikiversity" => {}
            "wikibooks" => {}
            _ => {
                return Err(
                    "Found invalid wiki in config 'wikis_to_include' while downloading data."
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn download_wikida_dump(url: &str, download_dir: &str) -> Result<(), String> {
    println!("Downloading wikidata dump from: {}", url);

    std::fs::create_dir_all(download_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let file_name = url.split('/').last().ok_or_else(|| {
        "Could not find file name in url while downloading wikida dump".to_string()
    })?;

    let destination_path = Path::new(download_dir).join(file_name);

    // Build the HTTP client with a User-Agent (to fix the 403 error)
    let client = reqwest::blocking::Client::builder()
        .user_agent("Offline Wikipedia Database")
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Server returned error status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .map_err(|e| format!("Failed to set progress bar style: {}", e))?
            .progress_chars("#>-"),
    );

    let mut dest_file =
        File::create(&destination_path).map_err(|e| format!("Failed to create file: {}", e))?;

    let mut source = pb.wrap_read(response);

    copy(&mut source, &mut dest_file)
        .map_err(|e| format!("Failed to write data to file: {}", e))?;

    pb.finish_with_message("Download complete!");

    Ok(())
}
fn download_wiki_from_url(url: &str, match_pattern: &str) -> Result<(), String> {
    Ok(())
}

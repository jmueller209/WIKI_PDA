use reqwest::Url;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::utils::constants;
use crate::utils::settings::Settings;

#[derive(Serialize, Deserialize)]
pub struct TagDictionaryCache {
    pub tags_version: Vec<String>,
    pub mapping: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct TagDictionaryMetrics {
    pub loaded_from_cache: bool,
    pub cache_status: String,
    pub total_mappings: usize,
    pub rate_limit_hits: u32,
    pub failed_tags: Vec<String>,
}

pub fn get_or_create_tag_dictionary(
    settings: &Settings,
) -> Result<(HashMap<String, Vec<String>>, TagDictionaryMetrics), Box<dyn std::error::Error>> {
    let mut all_tags: HashSet<String> = HashSet::new();

    all_tags.extend(
        settings
            .database_content
            .omni_search_index_tags
            .iter()
            .cloned(),
    );
    all_tags.extend(
        settings
            .database_content
            .astronomical_search_index_tags
            .iter()
            .cloned(),
    );
    all_tags.extend(
        settings
            .database_content
            .temporal_search_index_tags
            .iter()
            .cloned(),
    );
    all_tags.extend(
        settings
            .database_content
            .globe_coordinate_search_index_tags
            .iter()
            .cloned(),
    );

    let mut combined_tags: Vec<String> = all_tags.into_iter().collect();
    combined_tags.sort();

    let file_path = Path::new(&settings.paths.data_dir).join(constants::TAG_SUBCLASS_DICTIONARY_DB);
    let mut cache_status = "No dictionary found. Building for the first time...".to_string();

    if file_path.exists() {
        if let Ok(file) = File::open(&file_path) {
            let reader = BufReader::new(file);

            if let Ok(cached_data) = bincode::deserialize_from::<_, TagDictionaryCache>(reader) {
                if cached_data.tags_version == combined_tags {
                    let metrics = TagDictionaryMetrics {
                        loaded_from_cache: true,
                        cache_status: "Loaded valid tag dictionary from cache.".to_string(),
                        total_mappings: cached_data.mapping.len(),
                        rate_limit_hits: 0,
                        failed_tags: Vec::new(),
                    };
                    return Ok((cached_data.mapping, metrics));
                } else {
                    cache_status = "Configured tags changed. Rebuilding dictionary...".to_string();
                }
            } else {
                cache_status = "Cache corrupted or outdated. Rebuilding...".to_string();
            }
        }
    }

    let sparql_endpoint = "https://qlever.cs.uni-freiburg.de/api/wikidata";
    let (dictionary, fetch_metrics) = fetch_all_subclasses(&combined_tags, sparql_endpoint)
        .map_err(|e| Box::<dyn std::error::Error>::from(e))?;

    let cache = TagDictionaryCache {
        tags_version: combined_tags,
        mapping: dictionary.clone(),
    };

    let file = File::create(&file_path)?;
    let writer = BufWriter::new(file);

    bincode::serialize_into(writer, &cache)?;

    let final_metrics = TagDictionaryMetrics {
        loaded_from_cache: false,
        cache_status,
        total_mappings: dictionary.len(),
        rate_limit_hits: fetch_metrics.rate_limit_hits,
        failed_tags: fetch_metrics.failed_tags,
    };

    Ok((dictionary, final_metrics))
}

struct FetchMetrics {
    rate_limit_hits: u32,
    failed_tags: Vec<String>,
}

fn fetch_all_subclasses(
    parent_qids: &[String],
    endpoint_url: &str,
) -> Result<(HashMap<String, Vec<String>>, FetchMetrics), String> {
    let valid_qids: Vec<String> = parent_qids
        .iter()
        .filter(|qid| qid.starts_with('Q'))
        .cloned()
        .collect();

    if valid_qids.is_empty() {
        return Err("No valid Q-Identifiers provided.".to_string());
    }

    let mut subclass_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut rate_limit_hits = 0;
    let mut failed_tags = Vec::new();
    let total = valid_qids.len();

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    println!(
        "Fetching subclasses from Wikidata for {} distinct tags...",
        total
    );

    for (i, qid) in valid_qids.iter().enumerate() {
        print!("\r[{}/{}] Querying {}...          ", i + 1, total, qid);
        let _ = io::stdout().flush();

        let query = format!(
            "PREFIX wd: <http://www.wikidata.org/entity/> \
             PREFIX wdt: <http://www.wikidata.org/prop/direct/> \
             SELECT DISTINCT ?subclass WHERE {{ \
               ?subclass wdt:P279* wd:{}. \
             }}",
            qid
        );

        let url = match Url::parse_with_params(endpoint_url, &[("query", &query)]) {
            Ok(u) => u,
            Err(_) => {
                failed_tags.push(qid.clone());
                continue;
            }
        };

        let mut attempt = 1;
        let max_attempts = 5;
        let mut text_response = String::new();
        let mut request_successful = false;

        while attempt <= max_attempts {
            match client
                .get(url.clone())
                .header(ACCEPT, "application/sparql-results+json")
                .header(
                    USER_AGENT,
                    "WikiPDA-Generator/1.0 (https://github.com/yourusername/WIKI_PDA)",
                )
                .send()
            {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        if let Ok(text) = response.text() {
                            text_response = text;
                            request_successful = true;
                            break;
                        } else {
                            break;
                        }
                    } else if status.as_u16() == 429 {
                        rate_limit_hits += 1;
                        let backoff_secs = attempt * 5;

                        print!(
                            "\r[{}/{}] Rate limited on {}. Pausing for {}s...          ",
                            i + 1,
                            total,
                            qid,
                            backoff_secs
                        );
                        let _ = io::stdout().flush();

                        thread::sleep(Duration::from_secs(backoff_secs as u64));
                        attempt += 1;
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }

        if !request_successful {
            failed_tags.push(qid.clone());
            continue;
        }

        text_response.retain(|c| !c.is_control());

        if let Ok(json) = serde_json::from_str::<Value>(&text_response) {
            if let Some(bindings) = json["results"]["bindings"].as_array() {
                for row in bindings {
                    if let Some(subclass_url) = row["subclass"]["value"].as_str() {
                        let subclass_id = subclass_url.split('/').last().unwrap_or("").to_string();

                        if !subclass_id.is_empty() {
                            let parents = subclass_map.entry(subclass_id).or_insert_with(Vec::new);
                            if !parents.contains(qid) {
                                parents.push(qid.clone());
                            }
                        }
                    }
                }
            }
        } else {
            failed_tags.push(qid.clone());
        }

        thread::sleep(Duration::from_millis(500));
    }

    for qid in &valid_qids {
        let parents = subclass_map.entry(qid.clone()).or_insert_with(Vec::new);
        if !parents.contains(qid) {
            parents.push(qid.clone());
        }
    }

    println!(
        "\r[Done] Successfully prepared {} total unified subclass mappings!          ",
        subclass_map.len()
    );

    let metrics = FetchMetrics {
        rate_limit_hits,
        failed_tags,
    };

    Ok((subclass_map, metrics))
}


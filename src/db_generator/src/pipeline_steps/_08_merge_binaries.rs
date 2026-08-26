// use indicatif::{ProgressBar, ProgressStyle};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::utils::constants;
use crate::utils::settings::Settings;

const SD_CARD_SECTOR_SIZE: u64 = 512;

pub struct FileToMerge {
    pub key_name: String,
    pub path: PathBuf,
}

pub fn merge_into_master_database(settings: &Settings) -> Result<(), String> {
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let info_json_path = tmp_dir.join(constants::INFO_JSON);
    let master_db_path = bin_dir.join(constants::DATA_BASE_BIN);

    let mut info_json = load_json(&info_json_path)?;

    let files_to_merge = build_file_list(&bin_dir, &info_json, &settings)?;

    let file_info = merge_files(&files_to_merge, &master_db_path)?;

    update_info_json(&mut info_json, file_info)?;
    save_json(&info_json_path, &info_json)?;

    println!(
        "Master offsets and sizes successfully saved to {:?}",
        info_json_path
    );
    Ok(())
}

fn build_file_list(
    bin_dir: &Path,
    info_json: &Value,
    settings: &Settings,
) -> Result<Vec<FileToMerge>, String> {
    let mut files = Vec::new();

    let mut add_file = |key: &str, filename: &str| {
        files.push(FileToMerge {
            key_name: key.to_string(),
            path: bin_dir.join(filename),
        });
    };

    let num_sparse_levels = extract_sparse_levels(info_json, "omni_search")?;
    add_file("omni_search_level_0", constants::OMNI_SEARCH_BIN);
    for i in 1..=num_sparse_levels {
        let filename = constants::OMNI_SEARCH_SPARSE_INDEX_TEMPLATE_BIN
            .replace(".bin", &format!("_level_{}.bin", i));
        add_file(&format!("omni_search_level_{}", i), &filename);
    }

    if settings
        .database_content
        .create_globe_coordinate_search_index
    {
        let geo_levels = extract_sparse_levels(info_json, "globe_coordinate_search")?;
        add_file(
            "globe_coordinate_search_level_0",
            constants::GLOBE_COORDINATE_SEARCH_BIN,
        );
        for i in 1..=geo_levels {
            let filename = constants::GLOBE_COORDINATE_SEARCH_SPARSE_INDEX_TEMPLATE_BIN
                .replace(".bin", &format!("_level_{}.bin", i));
            add_file(&format!("globe_coordinate_search_level_{}", i), &filename);
        }
    }

    if settings.database_content.create_astronomical_search_index {
        let astro_levels = extract_sparse_levels(info_json, "astronomical_search")?;
        add_file(
            "astronomical_search_level_0",
            constants::ASTRONOMICAL_SEARCH_BIN,
        );
        for i in 1..=astro_levels {
            let filename = constants::ASTRONOMICAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN
                .replace(".bin", &format!("_level_{}.bin", i));
            add_file(&format!("astronomical_search_level_{}", i), &filename);
        }
    }

    if settings.database_content.create_temporal_search_index {
        let temporal_levels = extract_sparse_levels(info_json, "temporal_search")?;
        add_file("temporal_search_level_0", constants::TEMPORAL_SEARCH_BIN);
        for i in 1..=temporal_levels {
            let filename = constants::TEMPORAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN
                .replace(".bin", &format!("_level_{}.bin", i));
            add_file(&format!("temporal_search_level_{}", i), &filename);
        }
    }

    add_file("qid_hashmap", constants::QID_HASHMAP_BIN);
    add_file("qid_index", constants::QID_INDEX_BIN);
    add_file("titles", constants::TITLES_BIN);

    add_file("pid_hashmap", constants::PID_HASHMAP_BIN);
    add_file("pid_index", constants::PID_INDEX_BIN);
    add_file("pid_strings", constants::PID_STRINGS_BIN);

    add_file("content", constants::CONTENT_BIN);
    add_file("metadata", constants::META_DATA_BIN);
    add_file("zstd_dictionary", constants::ZSTD_DICTIONARY_BIN);

    Ok(files)
}

fn merge_files(
    files_to_merge: &[FileToMerge],
    output_path: &Path,
) -> Result<HashMap<String, (u64, u64)>, String> {
    let output_file = File::create(output_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(output_file);
    let mut file_info_map = HashMap::new();

    let mut current_offset: u64 = 0;
    let total_bytes = calculate_total_bytes(files_to_merge)?;

    let pb = indicatif::ProgressBar::new(total_bytes);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Stitching databases into {:?}...", output_path);

    for item in files_to_merge {
        let input_file =
            File::open(&item.path).map_err(|e| format!("Failed to open {:?}: {}", item.path, e))?;
        let actual_size = input_file.metadata().map_err(|e| e.to_string())?.len();

        file_info_map.insert(item.key_name.clone(), (current_offset, actual_size));

        let mut reader = BufReader::new(input_file);
        let mut buffer = [0u8; 8192];
        loop {
            let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            pb.inc(n as u64);
        }

        let padding_needed =
            (SD_CARD_SECTOR_SIZE - (actual_size % SD_CARD_SECTOR_SIZE)) % SD_CARD_SECTOR_SIZE;
        if padding_needed > 0 {
            let padding = vec![0u8; padding_needed as usize];
            writer.write_all(&padding).map_err(|e| e.to_string())?;
        }

        current_offset += actual_size + padding_needed;
    }

    writer.flush().map_err(|e| e.to_string())?;
    pb.finish_and_clear();
    println!("Successfully created combined database binary!");

    Ok(file_info_map)
}

fn update_info_json(
    root_json: &mut Value,
    file_info: HashMap<String, (u64, u64)>,
) -> Result<(), String> {
    let mut offsets = serde_json::Map::new();
    let mut sizes = serde_json::Map::new();

    let mut nested_offsets: HashMap<String, serde_json::Map<String, Value>> = HashMap::new();
    let mut nested_sizes: HashMap<String, serde_json::Map<String, Value>> = HashMap::new();

    for (key, (offset, size)) in file_info {
        if key.contains("_level_") {
            let parts: Vec<&str> = key.split("_level_").collect();
            if parts.len() == 2 {
                let index_name = parts[0];
                let level_num = parts[1];

                let root_key = format!("{}_level", index_name);
                let nested_key = format!("level_{}", level_num);

                nested_offsets
                    .entry(root_key.clone())
                    .or_default()
                    .insert(nested_key.clone(), json!(offset));
                nested_sizes
                    .entry(root_key)
                    .or_default()
                    .insert(nested_key, json!(size));
                continue;
            }
        }

        offsets.insert(key.clone(), json!(offset));
        sizes.insert(key, json!(size));
    }

    for (key, map) in nested_offsets {
        offsets.insert(key, json!(map));
    }
    for (key, map) in nested_sizes {
        sizes.insert(key, json!(map));
    }

    let root_obj = root_json
        .as_object_mut()
        .ok_or("info.json is not an object")?;
    root_obj.insert("offsets".to_string(), json!(offsets));
    root_obj.insert("sizes".to_string(), json!(sizes));

    Ok(())
}
fn load_json(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {:?}: {}", path, e))?;
    serde_json::from_reader(file).map_err(|e| format!("Failed to parse JSON: {}", e))
}

fn save_json(path: &Path, data: &Value) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("Failed to create JSON: {}", e))?;
    serde_json::to_writer_pretty(file, data).map_err(|e| format!("Failed to write JSON: {}", e))
}

fn extract_sparse_levels(json_val: &Value, key: &str) -> Result<u32, String> {
    json_val
        .get(key)
        .and_then(|v| v.get("num_sparse_levels"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .ok_or_else(|| format!("Missing '{}.num_sparse_levels' in info.json", key))
}

fn calculate_total_bytes(files: &[FileToMerge]) -> Result<u64, String> {
    let mut total = 0;
    for f in files {
        let meta =
            std::fs::metadata(&f.path).map_err(|e| format!("Missing file {:?}: {}", f.path, e))?;
        total += meta.len();
    }
    Ok(total)
}

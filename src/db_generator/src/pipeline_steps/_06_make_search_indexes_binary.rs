use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::settings::Settings;

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexMetadata {
    pub total_row_size: usize,
    pub term_size: usize,
    pub chunk_size_rows: u32,
    pub num_sparse_levels: u32,
    pub top_level_rows: u32,
}

impl IndexMetadata {
    pub fn empty() -> Self {
        Self {
            total_row_size: 0,
            term_size: 0,
            chunk_size_rows: 0,
            num_sparse_levels: 0,
            top_level_rows: 0,
        }
    }
}

#[derive(Debug)]
struct IndexConfig<'a> {
    name: &'a str,
    json_key: &'a str,
    is_enabled: bool,
    term_encoding_bytes: usize,
    txt_path: PathBuf,
    bin_path: PathBuf,
    tags: &'a [String],
    ram_limit_kb: usize,
    chunk_size_bytes: usize,
    sparse_index_template: &'a str,
}

pub fn make_binary_search_indexes(settings: &Settings) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 6) {
        checkpoints::CheckpointState::exists_empty => {
            println!("Checkpoint found: Binary index creation has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::exists_with_data(data) => {
            return Err(format!(
                "Binary index checkpoint should not contain data: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::exists_in_bad_state(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint found in bad state. Cleaned up checkpoints.".into());
        }
        checkpoints::CheckpointState::does_not_exist => (),
    }

    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);
    let info_json_path = tmp_dir.join(constants::INFO_JSON);

    let mut info_json = load_json(&info_json_path)?;

    let index_configs = vec![
        IndexConfig {
            name: "Omni Search",
            json_key: "omni_search",
            is_enabled: true, // Omni is always generated
            term_encoding_bytes: settings.performance.omni_search_index_term_encoding_bytes,
            txt_path: tmp_dir.join(constants::OMNI_SEARCH_TXT),
            bin_path: bin_dir.join(constants::OMNI_SEARCH_BIN),
            tags: &settings.database_content.omni_search_index_tags,
            ram_limit_kb: settings.performance.omni_search_sparse_index_ram_limit_kb,
            chunk_size_bytes: settings.performance.omni_search_chunk_size_bytes,
            sparse_index_template: constants::OMNI_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
        },
        IndexConfig {
            name: "Temporal Search",
            json_key: "temporal_search",
            is_enabled: settings.database_content.create_temporal_search_index,
            term_encoding_bytes: 4,
            txt_path: tmp_dir.join(constants::TEMPORAL_SEARCH_TXT),
            bin_path: bin_dir.join(constants::TEMPORAL_SEARCH_BIN),
            tags: &settings.database_content.temporal_search_index_tags,
            ram_limit_kb: settings.performance.temporal_serach_index_ram_limit_kb,
            chunk_size_bytes: settings.performance.temporal_search_chunk_size_bytes,
            sparse_index_template: constants::TEMPORAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
        },
        IndexConfig {
            name: "Astronomical Search",
            json_key: "astronomical_search",
            is_enabled: settings.database_content.create_astronomical_search_index,
            term_encoding_bytes: 4,
            txt_path: tmp_dir.join(constants::ASTRONOMICAL_SEARCH_TXT),
            bin_path: bin_dir.join(constants::ASTRONOMICAL_SEARCH_BIN),
            tags: &settings.database_content.astronomical_search_index_tags,
            ram_limit_kb: settings.performance.astronomical_search_index_ram_limit_kb,
            chunk_size_bytes: settings.performance.astronomical_search_chunk_size_bytes,
            sparse_index_template: constants::ASTRONOMICAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
        },
        IndexConfig {
            name: "Globe Coordinate Search",
            json_key: "globe_coordinate_search",
            is_enabled: settings
                .database_content
                .create_globe_coordinate_search_index,
            term_encoding_bytes: 4,
            txt_path: tmp_dir.join(constants::GLOBE_COORDINATE_SEARCH_TXT),
            bin_path: bin_dir.join(constants::GLOBE_COORDINATE_SEARCH_BIN),
            tags: &settings.database_content.globe_coordinate_search_index_tags,
            ram_limit_kb: settings
                .performance
                .globe_coordinate_search_index_ram_limit_kb,
            chunk_size_bytes: settings
                .performance
                .globe_coordinate_search_chunk_size_bytes,
            sparse_index_template: constants::GLOBE_COORDINATE_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
        },
    ];

    let mut enable_flags = serde_json::Map::new();
    for config in &index_configs {
        enable_flags.insert(
            config.json_key.to_string(),
            serde_json::Value::Bool(config.is_enabled),
        );

        if !config.is_enabled {
            println!("Skipping {} (Removing from info.json data)", config.name);
            if let Some(obj) = info_json.as_object_mut() {
                obj.remove(config.json_key);
            }
            continue;
        }

        println!("\n--- Processing {} ---", config.name);
        let meta_data = process_index_pipeline(config, &bin_dir, &settings.other.text_delimiter)?;
        info_json[config.json_key] = serde_json::to_value(&meta_data).map_err(|e| e.to_string())?;
    }

    info_json["wiki_pda_enable"] = serde_json::Value::Object(enable_flags);
    save_json(&info_json_path, &info_json)?;

    println!(
        "\nMetadata for all indexes successfully updated in {:?}",
        info_json_path
    );

    checkpoints::make_checkpoint(&settings, 6, "binary_index_creation", None)
        .map_err(|e| format!("Failed to create checkpoint: {}", e))?;

    Ok(())
}

fn process_index_pipeline(
    config: &IndexConfig,
    bin_dir: &Path,
    txt_delimiter: &str,
) -> Result<IndexMetadata, String> {
    let mut total_row_size = config.term_encoding_bytes + 8;
    if !total_row_size.is_power_of_two() {
        total_row_size = total_row_size.next_power_of_two();
    }
    let actual_term_bytes = total_row_size - 8;

    println!(
        "Requested term bytes: {} | Actual term bytes padded to power of 2: {}",
        config.term_encoding_bytes, actual_term_bytes
    );

    build_primary_binary(config, txt_delimiter, actual_term_bytes)?;

    let (num_levels, top_level_rows) = build_sparse_indexes(
        &config.bin_path,
        bin_dir,
        config.sparse_index_template,
        total_row_size,
        config.chunk_size_bytes,
        config.ram_limit_kb,
    )?;

    Ok(IndexMetadata {
        total_row_size,
        term_size: actual_term_bytes,
        chunk_size_rows: (config.chunk_size_bytes / total_row_size) as u32,
        num_sparse_levels: num_levels,
        top_level_rows,
    })
}

fn build_primary_binary(
    config: &IndexConfig,
    txt_delimiter: &str,
    actual_term_bytes: usize,
) -> Result<(), String> {
    let mut tag_to_bit: HashMap<String, u32> = HashMap::new();
    for (index, tag) in config.tags.iter().enumerate() {
        if index >= 32 {
            return Err("Cannot 1-hot encode more than 32 tags into a u32!".into());
        }
        tag_to_bit.insert(tag.clone(), 1 << index);
    }

    let input_file = File::open(&config.txt_path)
        .map_err(|e| format!("Failed to open {:?}: {}", config.txt_path, e))?;
    let file_size = input_file.metadata().map_err(|e| e.to_string())?.len();

    let reader = BufReader::new(input_file);
    let mut writer = BufWriter::new(File::create(&config.bin_path).map_err(|e| e.to_string())?);

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Compiling {} Binary...", config.name);

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        pb.inc((line.len() + 1) as u64);

        let parts: Vec<&str> = line.split(txt_delimiter).collect();
        if parts.len() < 2 {
            pb.println(format!("Warning: Skipping malformed line: {}", line));
            continue;
        }

        let search_term = parts[0];
        let qid_str = parts[1];
        let tags_str = if parts.len() > 2 { parts[2] } else { "" };

        let qid: u32 = if qid_str.starts_with('Q') {
            qid_str[1..].parse().unwrap_or(0)
        } else {
            qid_str.parse().unwrap_or(0)
        };

        let mut encoded_tags: u32 = 0;
        if !tags_str.is_empty() {
            for tag in tags_str.split(',') {
                if let Some(&bitmask) = tag_to_bit.get(tag.trim()) {
                    encoded_tags |= bitmask;
                }
            }
        }

        let mut term_bytes = vec![0u8; actual_term_bytes];

        if config.json_key == "omni_search" {
            let raw_bytes = search_term.as_bytes();
            let copy_len = raw_bytes.len().min(actual_term_bytes);
            term_bytes[..copy_len].copy_from_slice(&raw_bytes[..copy_len]);
        } else {
            let parsed_int: i64 = search_term.parse().unwrap_or_else(|_| {
                println!(
                    "Warning: Failed to parse '{}' as i64. Defaulting to 0.",
                    search_term
                );
                0
            });

            let raw_bytes = parsed_int.to_le_bytes(); // 8 bytes
            let copy_len = raw_bytes.len().min(actual_term_bytes);
            term_bytes[..copy_len].copy_from_slice(&raw_bytes[..copy_len]);
        }

        writer.write_all(&term_bytes).unwrap();
        writer.write_all(&qid.to_le_bytes()).unwrap();
        writer.write_all(&encoded_tags.to_le_bytes()).unwrap();
    }

    pb.finish_and_clear();
    writer.flush().map_err(|e| e.to_string())?;

    println!("Successfully built {} Binary!", config.name);
    Ok(())
}

fn build_sparse_indexes(
    initial_input_path: &Path,
    bin_dir: &Path,
    output_filename_template: &str,
    total_row_size: usize,
    chunk_size_bytes: usize,
    ram_limit_kb: usize,
) -> Result<(u32, u32), String> {
    if ram_limit_kb == 0 {
        println!("RAM limit is set to 0 KB. Skipping sparse index generation.");
        return Ok((0, 0));
    }

    let ram_limit_bytes = (ram_limit_kb * 1024) as u64;
    let string_bytes_len = total_row_size - 8;
    let chunk_size_rows = (chunk_size_bytes / total_row_size).max(1) as u32;

    let mut current_level = 1;
    let mut current_input_path: PathBuf = initial_input_path.to_path_buf();
    let final_top_level_rows: u32;

    println!(
        "Generating sparse indexes (Chunk size: {} rows / {} bytes, RAM Limit: {} KB)...",
        chunk_size_rows, chunk_size_bytes, ram_limit_kb
    );

    loop {
        let mut input_file = File::open(&current_input_path).map_err(|e| e.to_string())?;
        let input_size = input_file.metadata().map_err(|e| e.to_string())?.len();
        let total_input_rows = input_size / (total_row_size as u64);

        let output_filename =
            output_filename_template.replace(".bin", &format!("_level_{}.bin", current_level));
        let output_path = bin_dir.join(output_filename);
        let mut output_file =
            BufWriter::new(File::create(&output_path).map_err(|e| e.to_string())?);

        let mut row_index: u32 = 0;
        let mut term_buf = vec![0u8; string_bytes_len];

        println!("Building Level {} Index...", current_level);
        let pb = ProgressBar::new(total_input_rows);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] [{wide_bar:.green/blue}] {pos}/{len} rows ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        while (row_index as u64) < total_input_rows {
            let byte_offset = (row_index as u64) * (total_row_size as u64);
            input_file
                .seek(SeekFrom::Start(byte_offset))
                .map_err(|e| e.to_string())?;
            input_file
                .read_exact(&mut term_buf)
                .map_err(|e| e.to_string())?;

            output_file
                .write_all(&term_buf)
                .map_err(|e| e.to_string())?;
            output_file
                .write_all(&row_index.to_le_bytes())
                .map_err(|e| e.to_string())?;
            output_file
                .write_all(&[0u8; 4])
                .map_err(|e| e.to_string())?;

            row_index += chunk_size_rows;
            pb.set_position((row_index as u64).min(total_input_rows));
        }

        pb.finish_and_clear();
        output_file.flush().map_err(|e| e.to_string())?;

        let output_size = std::fs::metadata(&output_path)
            .map_err(|e| e.to_string())?
            .len();
        let current_level_rows = (output_size / (total_row_size as u64)) as u32;

        println!(
            "  -> Level {} generated: {} rows ({} bytes)",
            current_level, current_level_rows, output_size
        );

        if output_size <= ram_limit_bytes {
            println!(
                "Level {} fits strictly under {} KB. Sparse indexing complete!",
                current_level, ram_limit_kb
            );
            final_top_level_rows = current_level_rows;
            break;
        }

        current_input_path = output_path;
        current_level += 1;
    }

    Ok((current_level, final_top_level_rows))
}

fn load_json(path: &Path) -> Result<Value, String> {
    if path.exists() {
        let file = File::open(path).map_err(|e| format!("Failed to open {:?}: {}", path, e))?;
        Ok(serde_json::from_reader(file).unwrap_or_else(|_| serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

fn save_json(path: &Path, data: &Value) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("Failed to create JSON: {}", e))?;
    serde_json::to_writer_pretty(file, data).map_err(|e| format!("Failed to write JSON: {}", e))
}

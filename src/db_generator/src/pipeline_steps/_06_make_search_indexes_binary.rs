/* ============================================================================
 * UNIVERSAL SPARSE INDEX ARCHITECTURE (Omni, Temporal, Astro, Globe)
 * ============================================================================
 * The sparse indexes form a hierarchical B-Tree structure for fast lookups.
 * Level 1 points to chunks in the Main File (Level 0). Level 2 points to
 * chunks in Level 1, etc. The highest level fits entirely into the ESP32's RAM.
 *
 * To make C++ parsing extremely fast, EVERY sparse index row across ALL index
 * types uses the EXACT same `total_row_size` as its corresponding main index
 * (automatically padded to a power of 2, e.g., 16, 32, or 64 bytes).
 *
 * ----------------------------------------------------------------------------
 * GENERIC ROW LAYOUT:
 * ----------------------------------------------------------------------------
 * The layout math is always: `actual_term_bytes = total_row_size - 8`
 *
 * [ Bytes 0 to N-1 ] : Search Term (Size varies: 24 bytes for Omni, 8 for numeric)
 * [ Bytes N to N+3 ] : u32      - Target Row Index (Pointer to the layer below)
 * [ Bytes N+4 to N+7]: [u8; 4]  - Padding (0x00) for structural alignment
 *
 * ----------------------------------------------------------------------------
 * C++ STRUCTS FOR ESP32 (Examples based on typical sizes):
 * ----------------------------------------------------------------------------
 *
 * // 1. OMNI SEARCH (Example: 32-byte total row size, 24 actual_term_bytes)
 * struct __attribute__((packed)) OmniSparseRow {
 *     char search_term[24];  // String term, matches the main index
 *     uint32_t target_row;   // Row number in the file one level down
 *     uint32_t _padding;     // Ignored, keeps struct exactly 32 bytes
 * };
 *
 * // 2. NUMERIC SEARCH (Temporal / Astro / Globe - 16-byte total row size)
 * // A 4-byte u32 + 8 metadata bytes = 12. Padded to nearest power of 2 = 16.
 * // This means `actual_term_bytes` becomes 8 (4 bytes term + 4 bytes padding).
 * struct __attribute__((packed)) NumericSparseRow {
 *     uint32_t search_term;  // 4-byte search value (e.g., Year or Coordinate)
 *     uint32_t _term_pad;    // 4-byte padding to fill `actual_term_bytes`
 *     uint32_t target_row;   // Row number in the file one level down
 *     uint32_t _padding;     // 4-byte padding to fill total_row_size (16 bytes)
 * };
 *
 * ----------------------------------------------------------------------------
 * SEARCH LOGIC:
 * ----------------------------------------------------------------------------
 * When the ESP32 reads `target_row = 256`, it simply multiplies 256 by the
 * index's `total_row_size` (e.g., 32 or 16) to know exactly where to `fseek()`
 * in the binary file one level down.
 * ============================================================================
 */

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
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

#[derive(Debug)]
enum IndexType {
    Omni(String),
    GlobeCoordinate(String),
    Temporal(String),
    Astronomical(String),
}
#[derive(Debug)]
struct IndexSettings<'a> {
    name: String,
    term_encoding_bytes: usize,
    txt_path: PathBuf,
    bin_path: PathBuf,
    do_not_skip: bool,
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
                "Binary index checkpoint should not contain any data, but contains: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::exists_in_bad_state(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".to_string());
        }
        checkpoints::CheckpointState::does_not_exist => (),
    }

    let txt_delimiter = &settings.other.text_delimiter;
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let omni_search_index_txt_path = tmp_dir.join(constants::OMNI_SEARCH_TXT);
    let omni_search_index_bin_path =
        bin_dir.join(constants::OMNI_SEARCH_TXT.replace(".txt", ".bin"));

    let temporal_search_index_txt_path = tmp_dir.join(constants::TEMPORAL_SEARCH_TXT);
    let temporal_search_index_bin_path =
        bin_dir.join(constants::TEMPORAL_SEARCH_TXT.replace(".txt", ".bin"));

    let astronomical_search_index_txt_path = tmp_dir.join(constants::ASTRONOMICAL_SEARCH_TXT);
    let astronomical_search_index_bin_path =
        bin_dir.join(constants::ASTRONOMICAL_SEARCH_TXT.replace(".txt", ".bin"));

    let globe_coordinate_search_index_txt_path =
        tmp_dir.join(constants::GLOBE_COORDINATE_SEARCH_TXT);
    let globe_coordinate_search_index_bin_path =
        bin_dir.join(constants::GLOBE_COORDINATE_SEARCH_TXT.replace(".txt", ".bin"));

    let info_json_path = tmp_dir.join(constants::INFO_JSON);

    let omni_search_term_bytes = settings.performance.omni_search_index_term_encoding_bytes;
    let temporal_search_term_bytes = 4 as usize;
    let globel_coordiante_search_term_bytes = 4 as usize;
    let astronomical_search_term_bytes = 4 as usize;

    let omni_search_index_settings = IndexSettings {
        name: "Omni Search".to_string(),
        term_encoding_bytes: omni_search_term_bytes,
        txt_path: omni_search_index_txt_path,
        bin_path: omni_search_index_bin_path,
        do_not_skip: true,
        tags: &settings.database_content.omni_search_index_tags,
        ram_limit_kb: settings.performance.omni_search_sparse_index_ram_limit_kb,
        chunk_size_bytes: settings.performance.omni_search_chunk_size_bytes,
        sparse_index_template: constants::OMNI_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
    };

    let temporal_search_index_settings = IndexSettings {
        name: "Temporal Search".to_string(),
        term_encoding_bytes: temporal_search_term_bytes,
        txt_path: temporal_search_index_txt_path,
        bin_path: temporal_search_index_bin_path,
        do_not_skip: settings.database_content.create_temporal_search_index,
        tags: &settings.database_content.temporal_search_index_tags,
        ram_limit_kb: settings.performance.temporal_serach_index_ram_limit_kb,
        chunk_size_bytes: settings.performance.temporal_search_chunk_size_bytes,
        sparse_index_template: constants::TEMPORAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
    };

    let astronomical_search_index_settings = IndexSettings {
        name: "Astronomical Search".to_string(),
        term_encoding_bytes: astronomical_search_term_bytes,
        txt_path: astronomical_search_index_txt_path,
        bin_path: astronomical_search_index_bin_path,
        do_not_skip: settings.database_content.create_astronomical_search_index,
        tags: &settings.database_content.astronomical_search_index_tags,
        ram_limit_kb: settings.performance.astronomical_search_index_ram_limit_kb,
        chunk_size_bytes: settings.performance.astronomical_search_chunk_size_bytes,
        sparse_index_template: constants::ASTRONOMICAL_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
    };

    let globe_coordinate_search_index_settings = IndexSettings {
        name: "Globe Coordinate Search".to_string(),
        term_encoding_bytes: globel_coordiante_search_term_bytes,
        txt_path: globe_coordinate_search_index_txt_path,
        bin_path: globe_coordinate_search_index_bin_path,
        do_not_skip: settings
            .database_content
            .create_globe_coordinate_search_index,
        tags: &settings.database_content.globe_coordinate_search_index_tags,
        ram_limit_kb: settings
            .performance
            .globe_coordinate_search_index_ram_limit_kb,
        chunk_size_bytes: settings
            .performance
            .globe_coordinate_search_chunk_size_bytes,
        sparse_index_template: constants::GLOBE_COORDINATE_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
    };

    let index_settings_array: [IndexSettings; 4] = [
        omni_search_index_settings,
        temporal_search_index_settings,
        astronomical_search_index_settings,
        globe_coordinate_search_index_settings,
    ];

    for index_setting in &index_settings_array {
        if index_setting.do_not_skip == false {
            println!(
                "Skipping creation of {} Index as it was not requested.",
                index_setting.name
            );
            continue;
        }
        println!("Index Setting: {:?}", index_setting);
        let mut total_row_size = index_setting.term_encoding_bytes + 8;

        if !total_row_size.is_power_of_two() {
            total_row_size = total_row_size.next_power_of_two();
        }

        let actual_term_bytes = total_row_size - 8;

        println!("Requested term bytes for {} Index", &index_setting.name);
        println!(
            "Actual term bytes used to pad row size to a power of 2: {}",
            actual_term_bytes
        );

        let mut tag_to_bit: HashMap<String, u32> = HashMap::new();
        for (index, tag) in index_setting.tags.iter().enumerate() {
            if index >= 32 {
                return Err("Cannot 1-hot encode more than 32 tags into a u32!".into());
            }
            tag_to_bit.insert(tag.clone(), 1 << index);
        }

        let input_file = File::open(&index_setting.txt_path).map_err(|e| e.to_string())?;
        let file_size = input_file.metadata().map_err(|e| e.to_string())?.len();
        let reader = BufReader::new(input_file);
        let mut writer =
            BufWriter::new(File::create(&index_setting.bin_path).map_err(|e| e.to_string())?);

        let pb = ProgressBar::new(file_size);

        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "[{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        println!("Building {} Binary...", index_setting.name);

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
            let raw_bytes = search_term.as_bytes();

            let copy_len = raw_bytes.len().min(actual_term_bytes);
            term_bytes[..copy_len].copy_from_slice(&raw_bytes[..copy_len]);

            writer.write_all(&term_bytes).unwrap();
            writer.write_all(&qid.to_le_bytes()).unwrap();
            writer.write_all(&encoded_tags.to_le_bytes()).unwrap();
        }

        pb.finish_and_clear();
        println!("Successfully built {} Binary!", index_setting.name);

        writer.flush().map_err(|e| e.to_string())?;
        drop(writer);

        let (num_levels, top_level_rows) = build_sparse_indexes(
            &index_setting.bin_path,
            &bin_dir,
            index_setting.sparse_index_template,
            total_row_size,
            index_setting.chunk_size_bytes,
            index_setting.ram_limit_kb,
        )?;

        let meta_data = IndexMetadata {
            total_row_size,
            term_size: actual_term_bytes,
            chunk_size_rows: (index_setting.chunk_size_bytes / total_row_size) as u32,
            num_sparse_levels: num_levels,
            top_level_rows,
        };

        let mut root_json: serde_json::Value = if info_json_path.exists() {
            let file = File::open(&info_json_path).map_err(|e| e.to_string())?;
            serde_json::from_reader(file).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let root_name = index_setting.name.to_lowercase().replace(" ", "_");
        root_json[root_name] = serde_json::to_value(&meta_data).map_err(|e| e.to_string())?;
        let metadata_file = File::create(&info_json_path).map_err(|e| e.to_string())?;
        serde_json::to_writer_pretty(metadata_file, &root_json).map_err(|e| e.to_string())?;

        println!(
            "Metadata for {} index successfully updated in {:?}",
            index_setting.name, info_json_path
        );
    }

    checkpoints::make_checkpoint(&settings, 6, "binary_index_creation", None).map_err(|e| {
        format!(
            "Finished creating binary indexes, but failed to create checkpoint: {}",
            e
        )
    })?;

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

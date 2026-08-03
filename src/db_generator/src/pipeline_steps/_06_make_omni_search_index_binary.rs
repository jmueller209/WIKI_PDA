use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::utils::constants;
use crate::utils::settings::Settings;

#[derive(Serialize, Deserialize, Debug)]
pub struct OmniMetadata {
    pub total_row_size: usize,
    pub term_size: usize,
    pub chunk_size_rows: u32,
    pub num_sparse_levels: u32,
    pub top_level_rows: u32,
}

pub fn make_omni_search_index_binary(settings: &Settings) -> Result<(), String> {
    let txt_delimiter = &settings.other.text_delimiter;
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let omni_search_index_txt_path = tmp_dir.join(constants::OMNI_SEARCH_TXT);
    let omni_search_index_bin_path = bin_dir.join(constants::OMNI_SEARCH_BIN);
    let info_json_path = tmp_dir.join(constants::INFO_JSON);

    let requested_string_bytes = settings.performance.omni_search_index_term_encoding_bytes;
    let mut total_row_size = requested_string_bytes + 8;

    if !total_row_size.is_power_of_two() {
        total_row_size = total_row_size.next_power_of_two();
    }

    let actual_string_bytes = total_row_size - 8;

    println!("Requested term bytes: {}", requested_string_bytes);
    println!(
        "Actual term bytes (padded for alignment): {}",
        actual_string_bytes
    );
    println!("Total Row Size: {} bytes", total_row_size);

    let mut tag_to_bit: HashMap<String, u32> = HashMap::new();
    for (index, tag) in settings
        .database_content
        .omni_search_index_tags
        .iter()
        .enumerate()
    {
        if index >= 32 {
            return Err("Cannot 1-hot encode more than 32 tags into a u32!".into());
        }
        tag_to_bit.insert(tag.clone(), 1 << index);
    }

    let input_file = File::open(&omni_search_index_txt_path).map_err(|e| e.to_string())?;
    let file_size = input_file.metadata().map_err(|e| e.to_string())?.len();
    let reader = BufReader::new(input_file);
    let mut writer =
        BufWriter::new(File::create(&omni_search_index_bin_path).map_err(|e| e.to_string())?);

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Building Omni Search Binary...");

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
                if let Some(&bitmask) = tag_to_bit.get(tag) {
                    encoded_tags |= bitmask;
                }
            }
        }

        let mut term_bytes = vec![0u8; actual_string_bytes];
        let raw_bytes = search_term.as_bytes();

        let copy_len = raw_bytes.len().min(actual_string_bytes);
        term_bytes[..copy_len].copy_from_slice(&raw_bytes[..copy_len]);

        writer.write_all(&term_bytes).unwrap();
        writer.write_all(&qid.to_le_bytes()).unwrap();
        writer.write_all(&encoded_tags.to_le_bytes()).unwrap();
    }

    pb.finish_and_clear();
    println!("Successfully built Omni Search Binary!");

    let (num_levels, top_level_rows) = build_sparse_indexes(
        &omni_search_index_bin_path,
        &bin_dir,
        constants::OMNI_SEARCH_SPARSE_INDEX_TEMPLATE_BIN,
        total_row_size,
        settings.performance.omni_search_chunk_size_bytes,
        settings.performance.omni_search_sparse_index_ram_limit_kb,
    )?;

    let meta_data = OmniMetadata {
        total_row_size,
        term_size: actual_string_bytes,
        chunk_size_rows: (settings.performance.omni_search_chunk_size_bytes / total_row_size)
            as u32,
        num_sparse_levels: num_levels,
        top_level_rows,
    };

    let mut root_json: serde_json::Value = if info_json_path.exists() {
        let file = File::open(&info_json_path).map_err(|e| e.to_string())?;
        serde_json::from_reader(file).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    root_json["omni_search"] = serde_json::to_value(&meta_data).map_err(|e| e.to_string())?;
    let metadata_file = File::create(&info_json_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(metadata_file, &root_json).map_err(|e| e.to_string())?;

    println!("Metadata successfully updated in {:?}", info_json_path);

    Ok(())
}

/* ============================================================================
 * OMNI SEARCH SPARSE INDEX ARCHITECTURE
 * ============================================================================
 * The sparse indexes form a hierarchical B-Tree structure. Level 1 points to
 * chunks in the Main File (Level 0). Level 2 points to chunks in Level 1, etc.
 * The highest level fits entirely into the ESP32's RAM.
 *
 * To make C++ parsing extremely fast, every sparse index row uses the EXACT
 * same `total_row_size` as the main index (e.g., 32 or 64 bytes).
 *
 * Row Layout (Assuming a 32-byte total row size):
 * [ Bytes 0 to 23 ] : char[24] - Search Term (Matches the main file exactly)
 * [ Bytes 24 to 27] : u32      - Target Row Index (Pointer to the layer below)
 * [ Bytes 28 to 31] : [u8; 4]  - Padding (0x00) for structural alignment
 *
 * C++ STRUCT FOR ESP32 (Matches the size of OmniRow):
 * ----------------------------------------------------------------------------
 * struct __attribute__((packed)) OmniSparseRow {
 *     char search_term[24];  // Same size as your main index term
 *     uint32_t target_row;   // Row number in the file one level down
 *     uint32_t _padding;     // Ignored, keeps struct exactly 32 bytes
 * };
 *
 * SEARCH LOGIC:
 * When the ESP32 reads `target_row = 256`, it multiplies 256 by sizeof(OmniRow)
 * (e.g., 32 bytes) to know exactly where to `fseek()` in the file one level down.
 * ============================================================================
 */

//TODO: make this a general function that can be used for any index (astronomical, temporal, globe
//coordinate as well and put this into a shared module)
fn build_sparse_indexes(
    initial_input_path: &Path,
    bin_dir: &Path,
    output_filename_template: &str,
    total_row_size: usize,
    chunk_size_bytes: usize,
    ram_limit_kb: usize,
) -> Result<(u32, u32), String> {
    // <-- CHANGED

    if ram_limit_kb == 0 {
        println!("RAM limit is set to 0 KB. Skipping sparse index generation.");
        return Ok((0, 0)); // No levels, 0 top level rows
    }

    let ram_limit_bytes = (ram_limit_kb * 1024) as u64;
    let string_bytes_len = total_row_size - 8;

    let chunk_size_rows = (chunk_size_bytes / total_row_size).max(1) as u32;
    let mut current_level = 1;
    let mut current_input_path: PathBuf = initial_input_path.to_path_buf();

    // We will keep track of this during the loop
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

        // 1. Initialize the Progress Bar for this level
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

        // Calculate the rows for THIS level
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
            // Save the row count right before we break!
            final_top_level_rows = current_level_rows;
            break;
        }

        current_input_path = output_path;
        current_level += 1;
    }

    // Return both the level count and the number of rows in the top level
    Ok((current_level, final_top_level_rows))
}

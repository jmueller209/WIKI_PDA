use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;

/* ============================================================================
 * WIKIDATA PROPERTY (PID) BINARY INDEX ARCHITECTURE
 * ============================================================================
 *
 * FILE 1: pid_hashmap.bin (Direct-Mapped PID Lookup)
 * ----------------------------------------------------------------------------
 * This file acts as an O(1) lookup table. There are no actual PIDs stored here.
 * Instead, the PID itself is the row index. Missing PIDs are padded with zeros.
 *
 * Formula to find PID 'x':  file_offset = (x - 1) * 6 bytes
 *
 * Row Layout (6 bytes total, Little-Endian):
 * [ Bytes 0-3 ] : u32 - start_index (Row number in pid_index.bin where entries begin)
 * [ Bytes 4-5 ] : u16 - entry_count (Number of available translations/rows for this PID)
 *
 *
 * FILE 2: pid_index.bin (The Entries / Translations)
 * ----------------------------------------------------------------------------
 * This file holds the actual target data. Rows are strictly 10 bytes each.
 *
 * Formula to find Row 'y': file_offset = y * 10 bytes
 *
 * Row Layout (10 bytes total, Little-Endian):
 * [ Bytes 0-1 ] : u16 - project_id (Shares the same ID mapping as QIDs! e.g., 2=dewiki)
 * [ Bytes 2-5 ] : u32 - title_offset (Points to the label string in pid_strings.bin)
 * [ Bytes 6-9 ] : u32 - desc_offset (Points to the description string in pid_strings.bin)
 *
 *
 * FILE 3: pid_strings.bin (String Pool)
 * ----------------------------------------------------------------------------
 * Contains null-terminated UTF-8 strings. Byte 0 is always '\0'.
 * Strings are deduplicated during generation to save space.
 *
 *
 * C++ STRUCTS FOR ESP32:
 * ----------------------------------------------------------------------------
 * struct __attribute__((packed)) PropertyHashMapRow {
 *     uint32_t start_index;
 *     uint16_t entry_count;
 * }; // Exactly 6 bytes
 *
 * struct __attribute__((packed)) PropertyIndexRow {
 *     uint16_t project_id;
 *     uint32_t title_offset;
 *     uint32_t desc_offset;
 * }; // Exactly 10 bytes
 * ============================================================================
 */

// Helper function to deduplicate strings on the fly
fn get_string_offset(
    s: &str,
    pool: &mut HashMap<String, u32>,
    writer: &mut BufWriter<File>,
    current_offset: &mut u32,
) -> u32 {
    if s.trim().is_empty() {
        return 0;
    }
    if let Some(&offset) = pool.get(s) {
        return offset;
    }
    let offset = *current_offset;
    writer.write_all(s.as_bytes()).unwrap();
    writer.write_all(&[0u8]).unwrap();
    pool.insert(s.to_string(), offset);
    *current_offset += s.len() as u32 + 1;
    offset
}

pub fn make_pid_index_binary(settings: &Settings) -> Result<(), String> {
    // Assuming ID 6 for PID binary index checkpoint
    match checkpoints::checkpoint_exists(&settings, 6) {
        checkpoints::CheckpointState::exists_empty => {
            println!("Checkpoint found: Creation of the PID binary index has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::exists_with_data(data) => {
            return Err(format!(
                "Make PID binary index checkpoint should not contain any data, but contains: \n {}",
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

    // I assume these constants exist. If not, add them to constants.rs
    let pid_index_txt_path = tmp_dir.join(constants::PROPERTIES_SEARCH_TXT);
    let pid_hashmap_bin_path = bin_dir.join(constants::PID_HASHMAP_BIN);
    let pid_index_bin_path = bin_dir.join(constants::PID_INDEX_BIN);
    let pid_strings_bin_path = bin_dir.join(constants::PID_STRINGS_BIN);

    // The mapping created by the QID script
    let wiki_lang_mapping_txt_path = tmp_dir.join(constants::WIKI_LANG_MAPPING_TXT);

    // 1. LOAD GLOBAL PROJECT MAPPING (from QID generation)
    let mut project_dict: HashMap<String, u16> = HashMap::new();
    let mapping_file = File::open(&wiki_lang_mapping_txt_path).map_err(|e| {
        format!(
            "Could not open wiki_lang_mapping.txt. Did you run the QID script first? Error: {}",
            e
        )
    })?;

    for line_result in BufReader::new(mapping_file).lines() {
        let line = line_result.unwrap();
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() == 2 {
            let project_name = parts[0].to_string(); // Key
            let id: u16 = parts[1].parse().unwrap(); // Value
            project_dict.insert(project_name, id);
        }
    }
    println!(
        "Loaded {} existing project mappings for PIDs.",
        project_dict.len()
    );

    // 2. PREPARE FILES
    let input_file = File::open(&pid_index_txt_path)
        .map_err(|e| format!("Could not open PID index txt: {}", e))?;
    let file_size = input_file
        .metadata()
        .map_err(|e| format!("Could not read file metadata: {}", e))?
        .len();

    let reader = BufReader::new(input_file);

    let mut hashmap_writer = BufWriter::new(File::create(pid_hashmap_bin_path).unwrap());
    let mut index_writer = BufWriter::new(File::create(pid_index_bin_path).unwrap());
    let mut strings_writer = BufWriter::new(File::create(pid_strings_bin_path).unwrap());

    strings_writer.write_all(&[0u8]).unwrap();
    let mut current_string_offset: u32 = 1;
    let mut string_pool: HashMap<String, u32> = HashMap::new();

    let mut expected_pid: u32 = 1;
    let mut current_binary_row: u32 = 0;

    let mut current_pid_start: u32 = 0;
    let mut current_pid_count: u32 = 0;

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Building PID binary indexes...");

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        pb.inc((line.len() + 1) as u64);

        let parts: Vec<&str> = line.split(txt_delimiter).collect();
        if parts.len() < 3 {
            continue; // Skip malformed
        }

        // Parse PID ("P31" -> 31)
        let pid_num: u32 = match parts[0].trim_start_matches('P').parse() {
            Ok(val) => val,
            Err(_) => continue,
        };

        let lang_code = parts[1]; // e.g., "de"
        let project_name = format!("{}wiki", lang_code); // "dewiki"

        // Map to global project_id. If missing, skip this language translation.
        let project_id = match project_dict.get(&project_name) {
            Some(&id) => id,
            None => continue,
        };

        let title_str = parts[2];
        let desc_str = if parts.len() >= 4 { parts[3] } else { "" };

        // O(1) Padding: Fill gaps if PIDs were skipped
        while expected_pid < pid_num {
            assert!(
                current_pid_count <= u16::MAX as u32,
                "CRITICAL: entry_count exceeded u16 limits for a single PID!"
            );

            hashmap_writer
                .write_all(&current_pid_start.to_le_bytes())
                .unwrap();
            hashmap_writer
                .write_all(&(current_pid_count as u16).to_le_bytes())
                .unwrap();

            expected_pid += 1;
            current_pid_start = current_binary_row;
            current_pid_count = 0;
        }

        // Process strings with deduplication
        let row_title_offset = get_string_offset(
            title_str,
            &mut string_pool,
            &mut strings_writer,
            &mut current_string_offset,
        );
        let row_desc_offset = get_string_offset(
            desc_str,
            &mut string_pool,
            &mut strings_writer,
            &mut current_string_offset,
        );

        // Write 10 bytes to index.bin
        index_writer.write_all(&project_id.to_le_bytes()).unwrap();
        index_writer
            .write_all(&row_title_offset.to_le_bytes())
            .unwrap();
        index_writer
            .write_all(&row_desc_offset.to_le_bytes())
            .unwrap();

        current_binary_row += 1;
        current_pid_count += 1;
    }

    assert!(
        current_pid_count <= u16::MAX as u32,
        "CRITICAL: entry_count exceeded u16 limits for a single PID!"
    );
    hashmap_writer
        .write_all(&current_pid_start.to_le_bytes())
        .unwrap();
    hashmap_writer
        .write_all(&(current_pid_count as u16).to_le_bytes())
        .unwrap();

    pb.finish_and_clear();
    println!("Successfully built PID binary indexes!");
    println!("Total Index Rows (Translations): {}", current_binary_row);
    println!(
        "String Pool Size: {} bytes ({} unique strings)",
        current_string_offset,
        string_pool.len()
    );

    let summary_string = format!(
        "PID Binary Index Summary\nTotal Translations: {}\nString Pool: {} bytes\nMax PID: P{}",
        current_binary_row, current_string_offset, expected_pid
    );

    logs::write_summary_to_log(
        &summary_string,
        &settings,
        true,
        constants::MAKE_PID_BINARY_INDEX_LOG,
    )?;

    checkpoints::make_checkpoint(&settings, 6, "pid_binary_index_creation", None).map_err(|e| {
        format!(
            "Finished creating PID binary index, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

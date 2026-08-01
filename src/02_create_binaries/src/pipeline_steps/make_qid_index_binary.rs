use indicatif::{ProgressBar, ProgressStyle};
use shared::constants;
use shared::load_config::Settings;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

/* ============================================================================
 * WIKIDATA BINARY INDEX ARCHITECTURE
 * ============================================================================
 *
 * FILE 1: hashmap.bin (Direct-Mapped QID Lookup)
 * ----------------------------------------------------------------------------
 * This file acts as an O(1) lookup table. There are no actual QIDs stored here.
 * Instead, the QID itself is the row index. Missing QIDs are padded with zeros.
 *
 * Formula to find QID 'x':  file_offset = (x - 1) * 6 bytes
 *
 * Row Layout (6 bytes total, Little-Endian):
 * [ Bytes 0-3 ] : u32 - start_index (Row number in index.bin where entries begin)
 * [ Bytes 4-5 ] : u16 - entry_count (Number of sequential entries for this QID)
 *
 * Example: Reading QID 5
 * 1. Seek to byte (5 - 1) * 6 = 24 in hashmap.bin.
 * 2. Read 6 bytes.
 *    - If entry_count is 0, the QID does not exist in our dataset.
 *    - If start_index is 14 and entry_count is 2, go to row 14 in index.bin
 *      and read 2 rows.
 *
 *
 * FILE 2: index.bin (The Entries)
 * ----------------------------------------------------------------------------
 * This file holds the actual target data. Rows are strictly 14 bytes each.
 *
 * Formula to find Row 'y': file_offset = y * 14 bytes
 * (Note: y is exactly what you get from start_index above!)
 *
 * Row Layout (16 bytes total, Little-Endian):
 * [ Bytes 8-15]: u64 - offset (Absolute byte offset of payload in the data archive)
 * [ Bytes 4-7 ] : u32 - length (Size of the compressed payload in bytes)
 * [ Bytes 0-1 ] : u16 - project_id (Maps to project_dictionary.txt, e.g., 0=metadata)
 *
 *
 * C++ STRUCTS FOR ESP32:
 * ----------------------------------------------------------------------------
 * struct __attribute__((packed)) HashMapRow {
 *     uint32_t start_index; // Max 4.29 billion rows in index.bin
 *     uint16_t entry_count; // Max 65,535 languages/projects per QID
 * }; // Exactly 6 bytes
 *
 * struct __attribute__((packed)) IndexRow {
 *     uint64_t offset;
 *     uint32_t length;
 *     uint16_t project_id;
 * }; // Exactly 14 bytes
 * ============================================================================
 */

pub fn make_qid_index_binary(settings: &Settings) -> Result<(), String> {
    let txt_delimiter = &settings.other.text_delimiter;
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);
    let qid_index_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let qid_index_bin_path = bin_dir.join(constants::QID_INDEX_BIN);
    let qid_hashmap_bin_path = bin_dir.join(constants::QID_HASHMAP_BIN);
    let wiki_lang_mapping_txt_path = tmp_dir.join(constants::WIKI_LANG_MAPPING_TXT);

    // 2. Open the file and get its size for the progress bar
    let input_file = File::open(&qid_index_txt_path)
        .map_err(|e| format!("Could not open QID index txt: {}", e))?;
    let file_size = input_file
        .metadata()
        .map_err(|e| format!("Could not read file metadata: {}", e))?
        .len();

    let reader = BufReader::new(input_file);

    let mut hashmap_writer = BufWriter::new(File::create(qid_hashmap_bin_path).unwrap());
    let mut index_writer = BufWriter::new(File::create(qid_index_bin_path).unwrap());

    // Dictionary to assign u16 IDs to strings like "wiki_en"
    let mut project_dict: HashMap<String, u16> = HashMap::new();
    let mut next_project_id: u16 = 0;

    // Trackers for our direct-mapped array
    let mut expected_qid: u32 = 1;
    let mut current_binary_row: u32 = 0;

    let mut current_qid_start: u32 = 0;
    let mut current_qid_count: u32 = 0;

    // 3. Initialize the progress bar
    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Building QID binary indexes...");

    for line_result in reader.lines() {
        let line = line_result.unwrap();

        // 4. Increment the progress bar BEFORE any `continue` statements
        // +1 accounts for the newline character stripped by .lines()
        pb.inc((line.len() + 1) as u64);

        let parts: Vec<&str> = line.split(txt_delimiter).collect();
        if parts.len() != 4 {
            pb.println(format!("Warning: Skipping malformed line: {}", line));
            continue;
        }

        // Parse the QID (skip the 'Q' and parse the number)
        let qid_num: u32 = parts[0][1..].parse().unwrap();
        let project_str = parts[1];
        let offset: u64 = parts[2].parse().unwrap();
        let length: u32 = parts[3].parse().unwrap();

        // 1. Fill gaps in the hashmap if QIDs are missing or if the QID changed
        while expected_qid < qid_num {
            // SAFETY: Ensure we don't silently overflow our u16 limits
            assert!(
                current_qid_count <= u16::MAX as u32,
                "CRITICAL: entry_count exceeded u16 limits for a single QID!"
            );

            // Write the completed QID to the hashmap (Little Endian for ESP32)
            // 4 bytes for start_index
            hashmap_writer
                .write_all(&current_qid_start.to_le_bytes())
                .unwrap();
            // 2 bytes for entry_count
            hashmap_writer
                .write_all(&(current_qid_count as u16).to_le_bytes())
                .unwrap();

            expected_qid += 1;
            // The next QID's entries will start at whatever row we are currently on
            current_qid_start = current_binary_row;
            current_qid_count = 0;
        }

        // 2. Resolve the Project ID
        let project_id = *project_dict
            .entry(project_str.to_string())
            .or_insert_with(|| {
                let id = next_project_id;
                next_project_id += 1;
                id
            });

        // 3. Write the 16-byte entry to the binary index
        index_writer.write_all(&offset.to_le_bytes()).unwrap();
        index_writer.write_all(&length.to_le_bytes()).unwrap();
        index_writer.write_all(&project_id.to_le_bytes()).unwrap();

        current_binary_row += 1;
        current_qid_count += 1;
    }

    // Write the very last QID to the hashmap after the loop finishes
    assert!(
        current_qid_count <= u16::MAX as u32,
        "CRITICAL: entry_count exceeded u16 limits for a single QID!"
    );
    hashmap_writer
        .write_all(&current_qid_start.to_le_bytes())
        .unwrap();
    hashmap_writer
        .write_all(&(current_qid_count as u16).to_le_bytes())
        .unwrap();

    // 5. Clean up the progress bar
    pb.finish_and_clear();
    println!("Successfully built QID binary indexes!");

    // Finally, save the dictionary so your ESP32 knows what ID 0 or 1 means
    let mut dict_writer = BufWriter::new(File::create(wiki_lang_mapping_txt_path).unwrap());
    for (name, id) in project_dict {
        writeln!(dict_writer, "{}\t{}", id, name).unwrap();
    }

    Ok(())
}

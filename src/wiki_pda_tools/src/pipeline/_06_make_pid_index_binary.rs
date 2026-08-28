use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;

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
    println!("\n[DEBUG PAUSE]");
    println!("Please paste the correct properties_search.txt into the tmp folder.");
    print!("Press ENTER to continue...");
    io::stdout().flush().unwrap();

    let mut _dummy_input = String::new();
    io::stdin().read_line(&mut _dummy_input).unwrap();

    println!("Resuming script...\n");
    match checkpoints::checkpoint_exists(&settings, 6) {
        checkpoints::CheckpointState::ExistsEmpty => {
            println!("Checkpoint found: Creation of the PID binary index has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::ExistsWithData(data) => {
            return Err(format!(
                "Make PID binary index checkpoint should not contain any data, but contains: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::ExistsInBadState(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".to_string());
        }
        checkpoints::CheckpointState::DoesNotExist => (),
    }

    let txt_delimiter = &settings.other.text_delimiter;
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let pid_index_txt_path = tmp_dir.join(constants::PROPERTIES_SEARCH_TXT);
    let pid_hashmap_bin_path = bin_dir.join(constants::PID_HASHMAP_BIN);
    let pid_index_bin_path = bin_dir.join(constants::PID_INDEX_BIN);
    let pid_strings_bin_path = bin_dir.join(constants::PID_STRINGS_BIN);

    let wiki_lang_mapping_txt_path = tmp_dir.join(constants::WIKI_LANG_MAPPING_TXT);

    let mut lang_dict: HashMap<String, u16> = HashMap::new();
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
            let lang = parts[0].to_string();
            let id: u16 = parts[1].parse().unwrap();
            lang_dict.insert(lang, id);
        }
    }
    println!(
        "Loaded {} existing language mappings for PIDs.",
        lang_dict.len()
    );

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
            println!("DEBUG: Skipped due to delimiter! Line: {}", line);
            continue;
        }

        let pid_num: u32 = match parts[0].trim_start_matches('P').parse() {
            Ok(val) => val,
            Err(_) => {
                println!("DEBUG: Failed to parse PID: {}", parts[0]);
                continue;
            }
        };

        let lang_code = parts[1];

        let lang_id = match lang_dict.get(lang_code) {
            Some(&id) => id,
            None => {
                println!("DEBUG: Missing mapping for language: {}", lang_code);
                continue;
            }
        };

        let title_str = parts[2];
        let desc_str = if parts.len() >= 4 { parts[3] } else { "" };

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

        index_writer.write_all(&lang_id.to_le_bytes()).unwrap();
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

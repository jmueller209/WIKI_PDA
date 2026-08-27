use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;

pub fn make_qid_index_binary(settings: &Settings) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 5) {
        checkpoints::CheckpointState::ExistsEmpty => {
            println!("Checkpoint found: Creation of the QID binary index has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::ExistsWithData(data) => {
            return Err(format!(
                "Make QID binary index checkpoint should not contain any data, but contains: \n {}",
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

    let qid_index_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let qid_index_bin_path = bin_dir.join(constants::QID_INDEX_BIN);
    let qid_hashmap_bin_path = bin_dir.join(constants::QID_HASHMAP_BIN);
    let titles_bin_path = bin_dir.join(constants::TITLES_BIN);
    let wiki_lang_mapping_txt_path = tmp_dir.join(constants::WIKI_LANG_MAPPING_TXT);

    let input_file = File::open(&qid_index_txt_path)
        .map_err(|e| format!("Could not open QID index txt: {}", e))?;
    let file_size = input_file
        .metadata()
        .map_err(|e| format!("Could not read file metadata: {}", e))?
        .len();

    let reader = BufReader::new(input_file);

    let mut hashmap_writer = BufWriter::new(File::create(qid_hashmap_bin_path).unwrap());
    let mut index_writer = BufWriter::new(File::create(qid_index_bin_path).unwrap());
    let mut titles_writer = BufWriter::new(File::create(titles_bin_path).unwrap()); // <-- NEU

    titles_writer.write_all(&[0u8]).unwrap();
    let mut current_title_offset: u32 = 1;

    let mut language_dict: HashMap<String, u16> = HashMap::new();
    let mut next_language_id: u16 = 1;

    let mut expected_qid: u32 = 1;
    let mut current_binary_row: u32 = 0;

    let mut current_qid_start: u32 = 0;
    let mut current_qid_count: u32 = 0;

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

        pb.inc((line.len() + 1) as u64);

        let parts: Vec<&str> = line.split(txt_delimiter).collect();
        if parts.len() < 4 {
            pb.println(format!("Warning: Skipping malformed line: {}", line));
            continue;
        }

        let qid_num: u32 = parts[0][1..].parse().unwrap();
        let language_str = parts[1];
        let offset: u64 = parts[2].parse().unwrap();
        let length: u32 = parts[3].parse().unwrap();

        let title_str = if parts.len() >= 5 { parts[4] } else { "" };

        while expected_qid < qid_num {
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

            expected_qid += 1;
            current_qid_start = current_binary_row;
            current_qid_count = 0;
        }

        let lang_id = if language_str == "metadata" {
            0
        } else {
            *language_dict
                .entry(language_str.to_string())
                .or_insert_with(|| {
                    let id = next_language_id;
                    next_language_id += 1;
                    id
                })
        };

        let mut row_title_offset: u32 = 0;
        if language_str != "metadata" && !title_str.is_empty() {
            row_title_offset = current_title_offset;
            titles_writer.write_all(title_str.as_bytes()).unwrap();
            titles_writer.write_all(&[0u8]).unwrap();
            current_title_offset += title_str.len() as u32 + 1;
        }

        index_writer.write_all(&offset.to_le_bytes()).unwrap();
        index_writer.write_all(&length.to_le_bytes()).unwrap();
        index_writer.write_all(&lang_id.to_le_bytes()).unwrap();
        index_writer
            .write_all(&row_title_offset.to_le_bytes())
            .unwrap();

        current_binary_row += 1;
        current_qid_count += 1;
    }

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

    pb.finish_and_clear();
    println!("Successfully built QID binary indexes!");

    let mut dict_writer = BufWriter::new(File::create(wiki_lang_mapping_txt_path).unwrap());
    let mut sorted_languages: Vec<(String, u16)> = language_dict.into_iter().collect();
    sorted_languages.sort_by_key(|&(_, id)| id);
    for (lang, id) in sorted_languages {
        writeln!(dict_writer, "{}\t{}", lang, id).unwrap();
    }

    let summary_string = "Summary not available.";
    logs::write_summary_to_log(
        &summary_string,
        &settings,
        true,
        constants::MAKE_QID_BINARY_INDEX_LOG,
    )?;

    checkpoints::make_checkpoint(&settings, 5, "qid_binary_index_creation", None).map_err(|e| {
        format!(
            "Finished creating QID binary index, but failed to create checkpoint: {}",
            e
        )
    })?;

    Ok(())
}

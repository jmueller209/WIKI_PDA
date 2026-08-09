use indicatif::{ProgressBar, ProgressStyle};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::logs;
use crate::utils::settings::Settings;
use crate::utils::txt_file_processing::{self, SortMode};

pub fn make_metadata_binary(settings: &Settings) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 4) {
        checkpoints::CheckpointState::exists_empty => {
            println!("Checkpoint found: Generating metadata binary has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::exists_with_data(data) => {
            return Err(format!(
                "Checkpoint should not contain any data, but contains: \n {}",
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
    let thread_count = settings.performance.thread_count;
    let ram_limit_mb = settings.performance.ram_limit_mb;
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);
    let qid_index_txt_path = tmp_dir.join(constants::QID_INDEX_TXT);
    let metadata_txt_path = tmp_dir.join(constants::META_DATA_TXT);
    let metadata_bin_path = bin_dir.join(constants::META_DATA_TXT.replace(".txt", ".bin"));

    println!("Reading metadata from: {:?}", metadata_txt_path);

    let metadata_file = File::open(&metadata_txt_path)
        .map_err(|e| format!("Could not open metadata txt: {}", e))?;

    let file_size = metadata_file
        .metadata()
        .map_err(|e| format!("Could not read file metadata: {}", e))?
        .len();

    let reader = BufReader::new(metadata_file);

    let bin_file = File::create(&metadata_bin_path)
        .map_err(|e| format!("Could not create metadata bin: {}", e))?;
    let mut bin_writer = BufWriter::new(bin_file);

    let index_file = OpenOptions::new()
        .append(true)
        .open(&qid_index_txt_path)
        .map_err(|e| format!("Could not open qid index for appending: {}", e))?;
    let mut index_writer = BufWriter::new(index_file);

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut current_offset: u64 = 0;

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| e.to_string())?;

        pb.inc((line.len() + 1) as u64);

        if let Some((qid, content)) = line.split_once(txt_delimiter) {
            let bytes = content.as_bytes();
            let length = bytes.len() as u64;
            bin_writer
                .write_all(bytes)
                .map_err(|e| format!("Failed to write to bin file: {}", e))?;

            writeln!(
                index_writer,
                "{}{}metadata{}{}{}{}",
                qid, txt_delimiter, txt_delimiter, current_offset, txt_delimiter, length
            )
            .map_err(|e| format!("Failed to write to index file: {}", e))?;
            current_offset += length;
        } else {
            pb.println(format!(
                "Warning: Skipping malformed line missing delimiter: {}",
                line
            ));
        }
    }

    pb.finish_and_clear();

    bin_writer.flush().map_err(|e| e.to_string())?;
    index_writer.flush().map_err(|e| e.to_string())?;

    drop(index_writer);
    drop(bin_writer);

    println!("Successfully processed metadata and updated QID index.");

    txt_file_processing::external_merge_sort(
        qid_index_txt_path.to_str().unwrap(),
        qid_index_txt_path.to_str().unwrap(),
        SortMode::XId,
        ram_limit_mb,
        thread_count,
        &txt_delimiter,
    )
    .expect("Failed to sort QID Index");

    checkpoints::make_checkpoint(&settings, 4, "create_metadata_binary", None).map_err(|e| {
        format!(
            "Finished creating metadata binary, but failed to create checkpoint: {}",
            e
        )
    })?;

    let summary_string = "No summary available";
    logs::write_summary_to_log(
        &summary_string,
        &settings,
        true,
        constants::MAKE_METADATA_BINARY_LOG,
    )?;

    Ok(())
}

use crate::utils::settings::Settings;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

pub fn make_checkpoint(
    settings: &Settings,
    number: i8,
    title: &str,
    data: Option<&str>,
) -> Result<(), String> {
    let checkpoint_dir = Path::new(&settings.paths.checkpoint_dir);
    fs::create_dir_all(checkpoint_dir)
        .map_err(|e| format!("Failed to create checkpoint directory: {}", e))?;

    let filename = format!("checkpoint_{}_{}.cp", number, title);
    let file_path = checkpoint_dir.join(filename);

    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create checkpoint file at {:?}: {}", file_path, e))?;

    if let Some(content) = data {
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write data to checkpoint: {}", e))?;
    }
    println!("Created Checkpoint at {:?}.", file_path);
    Ok(())
}

pub enum CheckpointState {
    exists_empty,             // exists but does not contain any data
    exists_with_data(String), // exists and contains any data. Variant contains the data as
    // String
    exists_in_bad_state(i8), // The checkpoint exists but checkpoints before that checkpoint do
    // not exist. Variant holds the number of the
    // last valid checkpoint: E.g. we have
    // checkpoints 0, 1, 2, 3, 6, 7 -> The last
    // valid checkpoint is 3, because 4 is missing
    does_not_exist, // The checkpoint does not exist
}

pub fn checkpoint_exists(settings: &Settings, number: i8) -> CheckpointState {
    let checkpoint_dir = Path::new(&settings.paths.checkpoint_dir);

    let mut checkpoints = HashMap::new();

    if let Ok(entries) = fs::read_dir(checkpoint_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();

            if name_str.starts_with("checkpoint_") && name_str.ends_with(".cp") {
                let core = &name_str[11..name_str.len() - 3];
                if let Some((num_str, _title)) = core.split_once('_') {
                    if let Ok(num) = num_str.parse::<i8>() {
                        checkpoints.insert(num, entry.path());
                    }
                }
            }
        }
    }

    let file_path = match checkpoints.get(&number) {
        Some(path) => path,
        None => return CheckpointState::does_not_exist,
    };

    let mut last_valid: i8 = -1;
    let mut bad_state = false;

    for i in 0..number {
        if checkpoints.contains_key(&i) {
            last_valid = i;
        } else {
            bad_state = true;
            break;
        }
    }

    if bad_state {
        return CheckpointState::exists_in_bad_state(last_valid);
    }

    match fs::read_to_string(file_path) {
        Ok(content) => {
            if content.is_empty() {
                CheckpointState::exists_empty
            } else {
                CheckpointState::exists_with_data(content)
            }
        }
        Err(_) => CheckpointState::does_not_exist,
    }
}

pub fn clear_checkpoints(settings: &Settings, last_valid_checkpoint: i8) -> Result<(), String> {
    let checkpoint_dir = Path::new(&settings.paths.checkpoint_dir);

    if !checkpoint_dir.exists() {
        return Ok(());
    }

    if last_valid_checkpoint == -1 {
        return fs::remove_dir_all(checkpoint_dir)
            .map_err(|e| format!("Failed to remove checkpoint directory: {}", e));
    }

    let entries = fs::read_dir(checkpoint_dir)
        .map_err(|e| format!("Failed to read checkpoint directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        let mut should_delete = false;

        if let Some(core) = name_str
            .strip_prefix("checkpoint_")
            .and_then(|s| s.strip_suffix(".cp"))
        {
            if let Some((num_str, _title)) = core.split_once('_') {
                if let Ok(num) = num_str.parse::<i8>() {
                    if num > last_valid_checkpoint {
                        should_delete = true;
                    }
                } else {
                    should_delete = true;
                }
            } else {
                should_delete = true;
            }
        } else {
            should_delete = true;
        }

        if should_delete {
            if path.is_dir() {
                if let Err(e) = fs::remove_dir_all(&path) {
                    eprintln!("Failed to remove invalid directory {:?}: {}", path, e);
                }
            } else {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("Failed to remove file {:?}: {}", path, e);
                }
            }
        }
    }

    Ok(())
}

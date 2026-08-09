use std::fs;
use std::io;
use std::path::Path;

use crate::utils::checkpoints;
use crate::utils::settings::Settings;

pub fn clean(settings: &Settings) -> Result<(), String> {
    let tmp_dir = &settings.paths.tmp_dir;
    let log_dir = &settings.paths.log_dir;
    let bin_dir = &settings.paths.bin_dir;

    let dirs = [tmp_dir, log_dir, bin_dir];

    for dir in &dirs {
        remove_directory_safely(dir).map_err(|e| format!("Failed to remove directory: {}", e))?;
    }

    checkpoints::clear_checkpoints(&settings, 0)?;

    Ok(())
}

pub fn purge(settings: &Settings) -> Result<(), String> {
    clean(&settings)?;

    let data_dir = &settings.paths.data_dir;
    let checkpoint_dir = &settings.paths.checkpoint_dir;

    let dirs = [data_dir, checkpoint_dir];

    for dir in &dirs {
        remove_directory_safely(dir).map_err(|e| format!("Failed to remove directory: {}", e))?;
    }

    Ok(())
}

fn remove_directory_safely<P: AsRef<Path>>(path: P) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

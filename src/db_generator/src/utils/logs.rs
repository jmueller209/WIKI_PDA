use crate::utils::constants;
use crate::utils::settings::Settings;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn write_summary_to_log(
    summary: &str,
    settings: &Settings,
    print_to_terminal: bool,
    log_file: &str,
) -> Result<(), String> {
    if print_to_terminal == true {
        println!("{}", summary);
    }
    let log_base_path = &settings.paths.log_dir;

    let log_path = Path::new(log_base_path).join(log_file);

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).expect("Could not create logs directory!");
    }

    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open summary log file");

    log_file
        .write_all(summary.as_bytes())
        .expect("Failed to write summary to log file");

    println!("Saved Summary to {:?}", log_path);

    Ok(())
}

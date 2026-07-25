use shared::load_config::Paths;
use std::fs;
use std::path::Path;

pub fn remove_old_binaries(paths: &Paths) -> Result<(), std::io::Error> {
    println!("Restart flag detected. Deleting old binary and log files...");

    // 1. Delete the main binary file
    let bin_path = &paths.content_bin_file_path;
    if Path::new(bin_path).exists() {
        println!("  -> Removing {}", bin_path);
        fs::remove_file(bin_path)?;
    }

    // 2. Delete the unsorted index file
    // Crucial: We must apply the same name transformation as in zim_processor.rs!
    let idx_path = paths
        .qid_index_txt_file_path
        .replace(".txt", "_unsorted.txt");
    if Path::new(&idx_path).exists() {
        println!("  -> Removing {}", idx_path);
        fs::remove_file(&idx_path)?;
    }

    // 3. Delete the progression log
    let prog_path = &paths.progression_log_file_path;
    if Path::new(prog_path).exists() {
        println!("  -> Removing {}", prog_path);
        fs::remove_file(prog_path)?;
    }

    println!("Cleanup finished. Starting fresh.\n");
    Ok(())
}

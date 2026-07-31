use shared::constants;
use shared::load_config::Paths;
use std::fs;
use std::path::Path;

pub fn remove_old_binaries(paths: &Paths) -> Result<(), std::io::Error> {
    println!("Restart flag detected. Deleting old binary and log files...");

    // 1. Delete the main binary file holding the content
    let content_bin_path = Path::new(&paths.bin_dir).join(constants::CONTENT_BIN);
    if Path::new(&content_bin_path).exists() {
        println!("  -> Removing {:?}", content_bin_path);
        fs::remove_file(content_bin_path)?;
    }

    // 2. Delete the unsorted index file
    let qid_idx_unsorted_txt_path =
        Path::new(&paths.tmp_dir).join(constants::QID_INDEX_TXT.replace(".txt", "_unsorted.txt"));
    if Path::new(&qid_idx_unsorted_txt_path).exists() {
        println!("  -> Removing {:?}", qid_idx_unsorted_txt_path);
        fs::remove_file(qid_idx_unsorted_txt_path)?;
    }

    // 3. Delete the progression log
    let zim_progression_txt_path =
        Path::new(&paths.cache_dir).join(constants::ZIM_PROGRESSION_CACHE);
    if Path::new(&zim_progression_txt_path).exists() {
        println!("  -> Removing {:?}", zim_progression_txt_path);
        fs::remove_file(zim_progression_txt_path)?;
    }

    // 4. Delete the Zstd dictionary
    let zstd_dictionary_bin_path = Path::new(&paths.bin_dir).join(constants::ZSTD_DICTIONARY_BIN);
    if Path::new(&zstd_dictionary_bin_path).exists() {
        println!("  -> Removing {:?}", zstd_dictionary_bin_path);
        fs::remove_file(zstd_dictionary_bin_path)?;
    }

    println!("Cleanup finished. Starting fresh.\n");
    Ok(())
}

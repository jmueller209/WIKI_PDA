// use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::utils::constants;
use crate::utils::settings::Settings;


pub struct FileToMerge {
    pub key_name: String,
    pub path: PathBuf,
}

pub fn merge_into_master_database(settings: &Settings) -> Result<(), String> {
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let bin_dir = PathBuf::from(&settings.paths.bin_dir);

    let info_json_path = tmp_dir.join(constants::INFO_JSON);
    let data_base_bin_path = bin_dir.join(constants::DATA_BASE_BIN);

    let file = File::open(&info_json_path)
        .map_err(|e| format!("Failed to open info.json at {:?}: {}", info_json_path, e))?;
    let json_val: Value =
        serde_json::from_reader(file).map_err(|e| format!("Failed to parse info.json: {}", e))?;


    let num_sparse_levels = json_val
        .get("omni_search")
        .and_then(|v| v.get("num_sparse_levels"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            "Could not find 'num_sparse_levels' under 'omni_search' in info.json".to_string()
        })? as u32;


    let mut files_to_merge = Vec::new();


    let omni_search_index_bin_path = bin_dir.join(constants::OMNI_SEARCH_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "omni_search_level_0".to_string(),
        path: omni_search_index_bin_path,
    });


    for i in 1..=num_sparse_levels {
        let level_filename = constants::OMNI_SEARCH_SPARSE_INDEX_TEMPLATE_BIN
            .replace(".bin", &format!("_level_{}.bin", i));

        let path = bin_dir.join(level_filename);

        files_to_merge.push(FileToMerge {
            key_name: format!("omni_search_level_{}", i),
            path,
        });
    }


    let qid_hashmap_bin_path = bin_dir.join(constants::QID_HASHMAP_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "qid_hashmap".to_string(),
        path: qid_hashmap_bin_path,
    });


    let qid_index_bin_path = bin_dir.join(constants::QID_INDEX_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "qid_index".to_string(),
        path: qid_index_bin_path,
    });


    let content_bin_path = bin_dir.join(constants::CONTENT_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "content".to_string(),
        path: content_bin_path,
    });


    let metadata_bin_path = bin_dir.join(constants::META_DATA_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "metadata".to_string(),
        path: metadata_bin_path,
    });


    let zstd_dict_bin_path = bin_dir.join(constants::ZSTD_DICTIONARY_BIN);
    files_to_merge.push(FileToMerge {
        key_name: "zstd_dictionary".to_string(),
        path: zstd_dict_bin_path,
    });


    let file_info = merge_files(&files_to_merge, &data_base_bin_path)?;


    let mut root_obj = json_val
        .as_object()
        .cloned()
        .ok_or_else(|| "info.json root is not an object".to_string())?;


    let mut offsets_json_obj = serde_json::Map::new();
    let mut sizes_json_obj = serde_json::Map::new();

    let mut omni_search_level_offsets = serde_json::Map::new();
    let mut omni_search_level_sizes = serde_json::Map::new();


    for (key, (offset, size)) in file_info {
        if key.starts_with("omni_search_level_") {
            let nested_key = key.replace("omni_search_level_", "level_");

            omni_search_level_offsets
                .insert(nested_key.clone(), serde_json::Value::Number(offset.into()));
            omni_search_level_sizes.insert(nested_key, serde_json::Value::Number(size.into()));
        } else {
            offsets_json_obj.insert(key.clone(), serde_json::Value::Number(offset.into()));
            sizes_json_obj.insert(key, serde_json::Value::Number(size.into()));
        }
    }

    offsets_json_obj.insert(
        "omni_search_level".to_string(),
        serde_json::Value::Object(omni_search_level_offsets),
    );
    sizes_json_obj.insert(
        "omni_search_level".to_string(),
        serde_json::Value::Object(omni_search_level_sizes),
    );

    root_obj.insert(
        "offsets".to_string(),
        serde_json::Value::Object(offsets_json_obj),
    );
    root_obj.insert(
        "sizes".to_string(),
        serde_json::Value::Object(sizes_json_obj),
    );

    let out_file =
        File::create(&info_json_path).map_err(|e| format!("Failed to update info.json: {}", e))?;
    serde_json::to_writer_pretty(out_file, &root_obj)
        .map_err(|e| format!("Failed to write info.json: {}", e))?;

    println!(
        "Master offsets and sizes successfully saved to {:?}",
        info_json_path
    );

    Ok(())
}

fn merge_files(
    files_to_merge: &[FileToMerge],
    output_combined_path: &Path,
) -> Result<HashMap<String, (u64, u64)>, String> {
    let output_file = File::create(output_combined_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(output_file);

    let mut file_info = HashMap::new();
    let mut current_offset: u64 = 0;

    let mut total_bytes_to_merge: u64 = 0;
    for item in files_to_merge {
        if !item.path.exists() {
            return Err(format!("File to merge does not exist: {:?}", item.path));
        }
        let metadata = std::fs::metadata(&item.path).map_err(|e| e.to_string())?;
        total_bytes_to_merge += metadata.len();
    }

    let pb = indicatif::ProgressBar::new(total_bytes_to_merge);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.green/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    println!("Stitching databases into {:?}...", output_combined_path);

    for item in files_to_merge {
        let input_file = File::open(&item.path).map_err(|e| e.to_string())?;
        let file_size = input_file.metadata().map_err(|e| e.to_string())?.len();
        let mut reader = BufReader::new(input_file);

        file_info.insert(item.key_name.clone(), (current_offset, file_size));

        let mut buffer = [0u8; 8192];
        loop {
            let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n]).map_err(|e| e.to_string())?;

            pb.inc(n as u64);
        }

        current_offset += file_size;
    }

    writer.flush().map_err(|e| e.to_string())?;
    pb.finish_and_clear();

    println!("Successfully created combined database binary!");

    Ok(file_info)
}

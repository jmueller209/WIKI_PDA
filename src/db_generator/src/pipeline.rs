use crate::RunMode;

use crate::pipeline_steps::{
    _00_download_data, _01_parse_wikidata, _02_compression_setup, _03_process_zim_data,
    _04_make_metadata_binary, _05_make_qid_index_binary, _06_make_omni_search_index_binary,
    _07_merge_binaries, _08_make_c_header_file, _09_write_db_to_medium,
};
use crate::utils::settings::Settings;

pub fn run(settings: Settings, mode: RunMode) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        RunMode::Restart => {
            // println!("Starting fresh. Removing old files...");
            // cleanup::remove_old_binaries(&settings.paths)?;
            // println!("Generating zstd dictionary...");
            // preprocessor::generate_zstd_dictionary(&settings);
            //
            // println!("Starting ZIM extraction...");
            // zim_processor::process_directories(&settings)?;
            //
            // println!("Merging and finalizing indexes...");
            // index_merger::create_final_q_index(&settings)?;
        }
        RunMode::Resume => {
            // println!("Resume mode detected. Only processing missing ZIM files...");
            // println!("Generating zstd dictionary...");
            // preprocessor::generate_zstd_dictionary(&settings);
            //
            // println!("Starting ZIM extraction...");
            // zim_processor::process_directories(&settings)?;
            //
            // println!("Merging and finalizing indexes...");
            // index_merger::create_final_q_index(&settings)?;
        }
        RunMode::Test => {
            println!("Test mode detected");
            // _00_download_data::download_data(&settings)?;
            _01_parse_wikidata::parse_wikidata(&settings, Some(1000))?;
            // let _ = _02_compression_setup::generate_zstd_dictionary(&settings);
            // let _ = _03_process_zim_data::process_directories(&settings, Some(1000));
            // let _ = _04_make_metadata_binary::make_metadata_binary(&settings);
            // let _ = _05_make_qid_index_binary::make_qid_index_binary(&settings);
            // let _ = _06_make_omni_search_index_binary::make_omni_search_index_binary(&settings);
            // let _ = _07_merge_binaries::merge_into_master_database(&settings);
            // let _ = _08_make_c_header_file::make_c_header_file(&settings);
            //_09_write_db_to_medium::get_disks(&settings)?;
        }
    }

    Ok(())
}

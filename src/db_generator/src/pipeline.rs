use crate::RunMode;

use crate::tests::test_article_processing;

use crate::pipeline_steps::{
    _00_download_data, _01_parse_wikidata, _02_compression_setup, _03_process_zim_data,
    _04_make_metadata_binary, _05_make_qid_index_binary, _06_make_search_indexes_binary,
    _07_merge_binaries, _08_make_c_header_file, cleanup, disk_flasher,
};
use crate::utils::settings::Settings;

pub fn run(settings: Settings, mode: RunMode) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        RunMode::CleanAllExceptDownloads => {
            cleanup::clean(&settings)?;
        }
        RunMode::CleanAll => {
            cleanup::purge(&settings)?;
        }
        RunMode::RestartAllPurge => {
            cleanup::purge(&settings)?;
            run_all(&settings, None)?;
        }
        RunMode::RestartAllClean => {
            cleanup::clean(&settings)?;
            run_all(&settings, None)?;
        }
        RunMode::ResumeAll => {
            run_all(&settings, None)?;
        }

        RunMode::FlashDisk => {
            disk_flasher::cli(&settings)?;
        }

        RunMode::TestPipeline => {
            cleanup::clean(&settings)?;
            run_all(&settings, Some(10000))?;
        }
        RunMode::TestArticleProcessing => {
            test_article_processing::test_article_processing(&settings)?;
        }
        RunMode::Test => {
            println!("Test mode detected");
        }

        o => {
            println!("{:?} not implemented.", o);
        }
    }

    Ok(())
}

fn run_all(
    settings: &Settings,
    article_limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    _00_download_data::download_data(&settings)?;
    _01_parse_wikidata::parse_wikidata(&settings, article_limit)?;
    _02_compression_setup::generate_zstd_dictionary(&settings)?;
    _03_process_zim_data::process_directories(&settings, article_limit)?;
    _04_make_metadata_binary::make_metadata_binary(&settings)?;
    _05_make_qid_index_binary::make_qid_index_binary(&settings)?;
    _06_make_search_indexes_binary::make_binary_search_indexes(&settings)?;
    _07_merge_binaries::merge_into_master_database(&settings)?;
    _08_make_c_header_file::make_c_header_file(&settings)?;

    Ok(())
}

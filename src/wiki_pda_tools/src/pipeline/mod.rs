pub mod _00_download_data;
pub mod _01_parse_wikidata;
pub mod _02_compression_setup;
pub mod _03_process_zim_data;
pub mod _04_make_metadata_binary;
pub mod _05_make_qid_index_binary;
pub mod _06_make_pid_index_binary;
pub mod _07_make_search_indexes_binary;
pub mod _08_merge_binaries;
pub mod cleanup;

use crate::RunMode;
use crate::tools::test_article_processing;
use crate::utils::settings::Settings;

pub fn run(settings: Settings, mode: RunMode) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        RunMode::CleanAllExceptDownloads => cleanup::clean(&settings)?,
        RunMode::CleanAll => cleanup::purge(&settings)?,
        RunMode::RestartAllPurge => {
            cleanup::purge(&settings)?;
            run_all(&settings, None)?;
        }
        RunMode::RestartAllClean => {
            cleanup::clean(&settings)?;
            run_all(&settings, None)?;
        }
        RunMode::ResumeAll => run_all(&settings, None)?,

        RunMode::TestPipeline => {
            cleanup::clean(&settings)?;
            run_all(&settings, Some(2000))?;
        }
        RunMode::ExtractSampleArticles => {
            test_article_processing::extract_sample_articles(&settings)?
        }
        RunMode::TestArticleProcessing => {
            test_article_processing::test_article_processing(&settings)?
        }
        o => println!("{:?} not implemented.", o),
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
    _06_make_pid_index_binary::make_pid_index_binary(&settings)?;
    _07_make_search_indexes_binary::make_binary_search_indexes(&settings)?;
    _08_merge_binaries::merge_into_master_database(&settings)?;
    Ok(())
}

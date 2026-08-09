use std::env;
use std::process;

mod pipeline;
mod pipeline_steps;
mod tests;
mod utils;

use crate::utils::settings::Settings;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum RunMode {
    _00_Download,
    _01_WikidataParsing,
    _02_CompressionSetup,
    _03_ZimProcessing,
    _04_MetadataBinaryGeneration,
    _05_QID_IndexBinary,
    _06_SearchIndexesBinary,
    _07_MergeBinaries,
    RestartAllClean,
    RestartAllPurge,
    ResumeAll,
    CleanAll,
    CleanAllExceptDownloads,
    TestArticleProcessing,
    TestPipeline,
    Test,
}

pub enum PipelineStep {
    CleanAll,
    Download,
    ParseWikiData,
    TrainDictionary,
    CreateContent,
    MakeIndexes,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Should have provided 2 Arguments: config_path and mode. Found {}",
            args.len()
        );
        process::exit(1);
    }

    let config_path = &args[1];
    let mode_arg = &args[2];

    let run_mode = match mode_arg.as_str() {
        "--download" => RunMode::_00_Download,
        "--parse-wikidata" => RunMode::_01_WikidataParsing,
        "--train-dict" => RunMode::_02_CompressionSetup,
        "--process-zim" => RunMode::_03_ZimProcessing,
        "--metadata-bin" => RunMode::_04_MetadataBinaryGeneration,
        "--qid-bin" => RunMode::_05_QID_IndexBinary,
        "--search-bins" => RunMode::_06_SearchIndexesBinary,
        "--assemble" => RunMode::_07_MergeBinaries,
        "--clean" => RunMode::CleanAllExceptDownloads,
        "--purge" => RunMode::CleanAll,
        "--resume" => RunMode::ResumeAll,
        "--restart-clean" => RunMode::RestartAllClean,
        "--restart-purge" => RunMode::RestartAllPurge,
        "--test-pipeline" => RunMode::TestPipeline,
        "--test-article-processing" => RunMode::TestArticleProcessing,
        "--test" => RunMode::Test,
        _ => {
            eprintln!("Invalid run mode: {}", mode_arg);
            process::exit(1);
        }
    };

    let settings = Settings::load_from_file(config_path).unwrap_or_else(|err| {
        eprintln!(
            "Failed to load configuration from '{}': {}",
            config_path, err
        );
        process::exit(1);
    });

    println!("Loaded Config Successfully!");

    if let Err(e) = pipeline::run(settings, run_mode) {
        eprintln!("Fatal Error during binary creation: {}", e);
        process::exit(1);
    }

    println!("Finished!");
}

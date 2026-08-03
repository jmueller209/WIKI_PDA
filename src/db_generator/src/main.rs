use std::env;
use std::process;

mod pipeline;
mod pipeline_steps;
mod utils;

use crate::utils::settings::load_config_from_file;

pub enum RunMode {
    Resume,
    Restart,
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
    if args.len() < 3 {
        eprintln!("Usage: ./02_create_binaries <path_to_config.toml> [--resume | --restart]");
        process::exit(1);
    }

    let config_path = &args[1];
    let mode_arg = &args[2];

    let run_mode = match mode_arg.as_str() {
        "--resume" => RunMode::Resume,
        "--restart" => RunMode::Restart,
        "--test" => RunMode::Test,
        _ => {
            eprintln!(
                "Invalid run mode: {}. Use --resume, --restart, or --test.",
                mode_arg
            );
            process::exit(1);
        }
    };

    let settings = load_config_from_file(config_path).unwrap_or_else(|err| {
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

    println!("Binary creation completed successfully!");
}

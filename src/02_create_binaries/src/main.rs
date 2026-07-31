use shared::load_config;
use std::env;
use std::process;

mod cleanup;
mod index_merger;
mod pipeline;
mod preprocessor;
mod zim_processor;

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
        _ => {
            eprintln!("Fehler: Das dritte Argument muss '--resume' oder '--restart' sein.");
            process::exit(1);
        }
    };

    let settings = load_config::load_config_from_file(config_path).unwrap_or_else(|err| {
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

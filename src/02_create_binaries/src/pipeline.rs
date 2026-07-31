use crate::RunMode;
use crate::cleanup;
use crate::index_merger;
use crate::preprocessor;
use crate::zim_processor;
use shared::load_config::Settings;

pub fn run(settings: Settings, mode: RunMode) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        RunMode::Restart => {
            println!("Starting fresh. Removing old files...");
            cleanup::remove_old_binaries(&settings.paths)?;
        }
        RunMode::Resume => {
            println!("Resume mode detected. Only processing missing ZIM files...");
        }
    }

    println!("Generating zstd dictionary...");
    preprocessor::generate_zstd_dictionary(&settings);

    println!("Starting ZIM extraction...");
    zim_processor::process_directories(&settings)?;

    println!("Merging and finalizing indexes...");
    index_merger::create_final_q_index(&settings)?;

    Ok(())
}

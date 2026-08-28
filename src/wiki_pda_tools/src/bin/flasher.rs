use std::env;
use std::process;

use wiki_pda_tools::tools::disk_flasher;
use wiki_pda_tools::utils::settings::Settings;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: flasher <config_path>");
        process::exit(1);
    }

    let config_path = &args[1];

    let settings = Settings::load_from_file(config_path).unwrap_or_else(|err| {
        eprintln!(
            "Failed to load configuration from '{}': {}",
            config_path, err
        );
        process::exit(1);
    });

    if let Err(e) = disk_flasher::cli(&settings) {
        eprintln!("Fatal Error during flashing: {}", e);
        process::exit(1);
    }
}

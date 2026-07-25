use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_file = PathBuf::from(&crate_dir).join("cbindgen.toml");
    let output_file = PathBuf::from(&crate_dir).join("encodings.h");

    let config = cbindgen::Config::from_file(&config_file).expect("Failed to read cbindgen.toml");

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Failed to generate bindings")
        .write_to_file(output_file);
}

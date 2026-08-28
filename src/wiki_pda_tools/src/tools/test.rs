use crate::utils::settings::Settings;
// use std::env;
// use std::panic;
// use std::process::Command;

pub fn test(_settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    reproduce_error();
    Ok(())
}

fn reproduce_error() {
    let html = r###"/div>

    <table class="wikitable center" style="text-align:center" id="mwMg">
    <td rowspan="00" id="mwOw"><"###;

    // Trigger the panic
    let _ = html2text::config::plain().string_from_read(html.as_bytes(), 10000);
}

// fn get_rust_version() -> String {
//     if let Ok(output) = Command::new("rustc").arg("--version").output() {
//         if let Ok(version) = String::from_utf8(output.stdout) {
//             return version.trim().to_string();
//         }
//     }
//     "Unknown".to_string()
// }
//
// fn generate_issue_markdown(html: &str, panic_msg: &str) -> String {
//     let rust_version = get_rust_version();
//     let os_name = env::consts::OS;
//     let arch = env::consts::ARCH;
//
//     let template = vec![
//         "**Title:** Panic: `{panic_msg}` when parsing HTML snippet",
//         "",
//         "### Description",
//         "When parsing a specific, highly nested and malformed HTML snippet (extracted from a Wikipedia dump), the library panics.",
//         "",
//         "### Minimal Reproducible Example",
//         "I am currently building an offline Wikipedia Database. Therefore, I am parsing a lot of html and occasionally encounter errors including hard panics which should not happen. I built a debugger to minimize a larger HTML file down to the exact snippet that triggers the panic. Running this simple code crashes the parser:",
//         "",
//         "```rust",
//         "fn main() {",
//         "    let html = r###\"{html}\"###;",
//         "",
//         "    // Trigger the panic",
//         "    let _ = html2text::config::plain().string_from_read(html.as_bytes(), 10000);",
//         "}",
//         "```",
//         "",
//         "### Actual Behavior",
//         "The code panics immediately. The panic message is:",
//         "",
//         "```text",
//         "thread 'main' panicked at '{panic_msg}'",
//         "```",
//         "",
//         "### Expected Behavior",
//         "The library should handle the malformed HTML gracefully (e.g., by rendering it poorly or ignoring the broken tags) or return a `Result::Err`, rather than causing a hard panic.",
//         "",
//         "### Environment",
//         "* **html2text version:** 0.17.1",
//         "* **Rust version:** {rust_version}",
//         "* **OS:** {os_name} ({arch})",
//     ].join("\n");
//
//     template
//         .replace("{panic_msg}", panic_msg)
//         .replace("{html}", html)
//         .replace("{rust_version}", &rust_version)
//         .replace("{os_name}", os_name)
//         .replace("{arch}", arch)
// }


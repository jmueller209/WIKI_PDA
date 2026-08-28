use crate::utils::settings::Settings;
use std::env;
use std::panic;
use std::process::Command;

pub fn test(_settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let html = r###"/p>
<table cellpadding="0" style="border-style:none;font-size:90%;border-collapse:separate;border-spacing:0;margin:1em 2em 1em 1em"><td colspan="3" style="text-align:center;border:1px solid var(--border-color-base,#a2a9b1);color:var(--color-base,#202122);overflow:inherit;background-color:var(--background-color-neutral,#eaecf0)"><td rowspan="3" colspan="5" style="border-style:solid;border-width:0;border-color:inherit"><tr><td rowspan="2" style="border-style:solid;border-width:0;border-color:inherit;border-bottom-width:0"><tr><td rowspan="2" style="border-style:solid;border-width:0;border-color:inherit;border-top-width:0"><tr><td style="height:7px"><td rowspan="2" style="color:var(--color-base,#202122);overflow:inherit;background-color:var(--background-color-neutral-subtle,#f8f9fa);padding:0 2px;border:1px solid var(--border-color-base,#a2a9b1);border-top-width:1px"><b>NY Rangers<td rowspan="2" colspan="8" style="text-align:center"><b><a href="Eastern_Conference_(NHL)" title="Eastern Conference (NHL)">Eastern Conference<tr><td style="height:7px"><td style="border-color:inherit;border-width:0 0px 0 0;border-style:solid"><"###;

    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let result = panic::catch_unwind(|| {
        reproduce_error(html);
    });

    panic::set_hook(original_hook);

    if let Err(panic_payload) = result {
        let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        let issue_text = generate_issue_markdown(html, &panic_msg);
        println!("{}", issue_text);
    } else {
        println!("Test bestanden! Dieser HTML-String löst keinen Crash aus.");
    }

    Ok(())
}

fn reproduce_error(html: &str) {
    let _ = html2text::from_read(html.as_bytes(), 10000);
}

fn get_rust_version() -> String {
    if let Ok(output) = Command::new("rustc").arg("--version").output() {
        if let Ok(version) = String::from_utf8(output.stdout) {
            return version.trim().to_string();
        }
    }
    "Unknown".to_string()
}

fn generate_issue_markdown(html: &str, panic_msg: &str) -> String {
    let rust_version = get_rust_version();
    let os_name = env::consts::OS;
    let arch = env::consts::ARCH;

    let template = vec![
        "**Title:** Panic: `{panic_msg}` when parsing HTML snippet",
        "",
        "### Description",
        "When parsing a specific, highly nested and malformed HTML snippet (extracted from a Wikipedia dump), the library panics.",
        "",
        "### Minimal Reproducible Example",
        "I built a delta-debugger to minimize a larger HTML file down to the exact snippet that triggers the panic. Running this simple code reliably crashes the parser:",
        "",
        "```rust",
        "fn main() {",
        "    let html = r###\"{html}\"###;",
        "",
        "    // Trigger the panic",
        "    let _ = html2text::from_read(html.as_bytes(), 10000);",
        "}",
        "```",
        "",
        "### Actual Behavior",
        "The code panics immediately. The panic message is:",
        "",
        "```text",
        "thread 'main' panicked at '{panic_msg}'",
        "```",
        "",
        "### Expected Behavior",
        "The library should handle the malformed HTML gracefully (e.g., by rendering it poorly or ignoring the broken tags) or return a `Result::Err`, rather than causing a hard panic.",
        "",
        "### Environment",
        "* **html2text version:** 0.17.1",
        "* **Rust version:** {rust_version}",
        "* **OS:** {os_name} ({arch})",
    ].join("\n");

    template
        .replace("{panic_msg}", panic_msg)
        .replace("{html}", html)
        .replace("{rust_version}", &rust_version)
        .replace("{os_name}", os_name)
        .replace("{arch}", arch)
}


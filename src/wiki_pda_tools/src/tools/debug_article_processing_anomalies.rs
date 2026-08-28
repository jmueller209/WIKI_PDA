use serde::Deserialize;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::utils::article_processing::{ArticleProcessor, DefaultArticleProcessor};
use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;

#[derive(Deserialize, Debug)]
struct Anomaly {
    qid: String,
    lang: String,
    title: String,
    error_msg: String,
    raw_content: String,
}

pub fn run_anomaly_analyzer(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = PathBuf::from(&settings.paths.log_dir);
    let anomalies_path = log_dir.join("anomalies.jsonl");

    if !anomalies_path.exists() {
        println!("No anomalies.jsonl found at {:?}", anomalies_path);
        return Ok(());
    }

    let file = File::open(&anomalies_path)?;
    let reader = BufReader::new(file);
    let mut anomalies: Vec<Anomaly> = Vec::new();

    for line in reader.lines() {
        if let Ok(l) = line {
            if let Ok(anomaly) = serde_json::from_str::<Anomaly>(&l) {
                anomalies.push(anomaly);
            }
        }
    }

    if anomalies.is_empty() {
        println!("The anomalies.jsonl is empty. Everything is fine!");
        return Ok(());
    }

    println!("Opening sitelinks database for detailed analysis...");
    let db = sitelinks_lookup::open_sitelinks_db(settings);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(sitelinks_lookup::SITELINKS_TABLE)?;

    let mut current_idx = 0;
    loop {
        let anomaly = &anomalies[current_idx];

        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        println!("==================================================");
        println!("ANOMALY {} / {}", current_idx + 1, anomalies.len());
        println!("==================================================");
        println!("QID:   {}", anomaly.qid);
        println!("Lang:  {}", anomaly.lang);
        println!("Title: {}", anomaly.title);
        println!("Error: {}", anomaly.error_msg);
        println!("--------------------------------------------------");

        let max_chars = 150;
        let snippet: String = anomaly.raw_content.chars().take(max_chars).collect();
        println!("Content Snippet:\n{} ...", snippet);
        println!("(Total size: {} bytes)", anomaly.raw_content.len());
        println!("==================================================");

        println!("\nActions:");
        println!(" [n] Next error");
        println!(" [p] Previous error");
        println!(" [m] Pinpoint error exactly (Minimize / Bisection)");
        println!(" [q] Quit");
        print!("\nInput: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().to_lowercase().as_str() {
            "n" => {
                if current_idx + 1 < anomalies.len() {
                    current_idx += 1;
                }
            }
            "p" => {
                if current_idx > 0 {
                    current_idx -= 1;
                }
            }
            "m" => {
                println!("\nStarting minimization (This may take a few seconds)...");
                let minimal_html = minimize_crash(anomaly, settings, &table);
                println!("\n=== MINIMAL HTML TRIGGERING THE ERROR ===");
                println!("{}", minimal_html);
                println!("==============================================");
                println!("Press Enter to continue...");
                let _ = io::stdin().read_line(&mut String::new());
            }
            "q" => {
                println!("Quit.");
                break;
            }
            _ => println!("Invalid input."),
        }
    }

    Ok(())
}

fn triggers_error(
    html: &str,
    anomaly: &Anomaly,
    settings: &Settings,
    table: &redb::ReadOnlyTable<&str, &str>,
) -> bool {
    let processor = DefaultArticleProcessor;
    let mut search_key_buffer = String::with_capacity(256);

    processor
        .process(
            &anomaly.qid,
            html,
            table,
            &mut search_key_buffer,
            settings,
            &anomaly.lang,
        )
        .is_err()
}

fn minimize_crash(
    anomaly: &Anomaly,
    settings: &Settings,
    table: &redb::ReadOnlyTable<&str, &str>,
) -> String {
    let mut tokens: Vec<&str> = anomaly.raw_content.split_inclusive('<').collect();

    if !triggers_error(&tokens.join(""), anomaly, settings, table) {
        return "ERROR: The original HTML does not throw an error in isolation anymore!"
            .to_string();
    }

    let mut chunk_size = tokens.len() / 2;

    while chunk_size > 0 {
        let mut i = 0;
        let mut reduced = false;

        while i + chunk_size <= tokens.len() {
            let mut test_tokens = tokens[..i].to_vec();
            test_tokens.extend_from_slice(&tokens[i + chunk_size..]);

            let test_html = test_tokens.join("");

            if triggers_error(&test_html, anomaly, settings, table) {
                tokens = test_tokens;
                reduced = true;
                print!("\r - Reduced! ({} fragments remaining)   ", tokens.len());
                io::stdout().flush().unwrap();
            } else {
                i += chunk_size;
            }
        }

        if !reduced {
            chunk_size /= 2;
        }
    }

    println!();
    tokens.join("")
}

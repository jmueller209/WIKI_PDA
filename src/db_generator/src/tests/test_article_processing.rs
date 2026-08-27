use redb::ReadOnlyTable;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::article_processing;
use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;

// ============================================================================
// HILFSFUNKTIONEN FÜR DIE EXTRAKTION
// ============================================================================

/// Sucht die passende ZIM-Datei für eine bestimmte Sprache im Verzeichnis
fn find_zim_file(dir: &Path, pattern: &str, target_lang: &str) -> Option<PathBuf> {
    let regex_str = pattern.replace("{lang}", "(?P<lang>[a-zA-Z-]+)");
    let re = Regex::new(&regex_str).ok()?;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            if let Some(captures) = re.captures(&file_name) {
                if let Some(lang_match) = captures.name("lang") {
                    if lang_match.as_str() == target_lang {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Extrahiert die angegebenen Test-Artikel aus der ZIM-Datei und speichert sie als HTML
fn extract_from_zim(zim_path: &Path, lang: &str, out_dir: &Path, titles: &[&str]) {
    let zim_file = match zim::Zim::new(zim_path) {
        Ok(z) => z,
        Err(e) => {
            println!("   -> Failed to open ZIM {:?}: {}", zim_path, e);
            return;
        }
    };

    let mut remaining = titles.to_vec();

    for direntry_result in zim_file.iterate_by_urls() {
        if remaining.is_empty() {
            break; // Alle gefunden!
        }

        let direntry = match direntry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !matches!(
            direntry.namespace,
            zim::Namespace::Articles | zim::Namespace::UserContent
        ) {
            continue;
        }

        let raw_string = if !direntry.title.is_empty() {
            direntry.title.as_str()
        } else {
            &direntry.url
        };

        let decoded_url =
            urlencoding::decode(raw_string).unwrap_or(std::borrow::Cow::Borrowed(raw_string));
        let clean_title = decoded_url.replace('_', " ");

        if let Some(pos) = remaining
            .iter()
            .position(|&t| clean_title.eq_ignore_ascii_case(t))
        {
            if let Ok(Some(content)) = zim_file.entry_content(&direntry) {
                let raw_text = content
                    .with(|bytes| unsafe { std::str::from_utf8_unchecked(bytes).to_string() })
                    .unwrap_or_default();

                let safe_title = remaining[pos].replace(['/', '\\', ' '], "_");
                let out_filename = format!("sample_wiki_{}_{}.html", lang, safe_title);
                let out_path = out_dir.join(&out_filename);

                if let Err(e) = fs::write(&out_path, raw_text) {
                    println!("   -> Failed to save {:?}: {}", out_path, e);
                } else {
                    println!("   -> Saved raw article: {:?}", out_path);
                }

                remaining.remove(pos);
            }
        }
    }

    if !remaining.is_empty() {
        println!(
            "   -> Warning: Could not find these articles: {:?}",
            remaining
        );
    }
}

// ============================================================================
// HILFSFUNKTION FÜR DAS TESTEN (PARSEN)
// ============================================================================

/// Liest eine extrahierte HTML-Datei, jagt sie durch den Parser und speichert das .txt Resultat
fn test_single_article(
    path: &Path,
    out_dir: &Path,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let parts: Vec<&str> = filename.splitn(4, '_').collect();

    // Ignoriere Dateien, die nicht unserem Namensschema entsprechen
    if parts.len() < 4 || parts[0] != "sample" {
        return Ok(());
    }

    let wiki_type = parts[1];
    let lang = parts[2];
    let title_with_ext = parts[3];
    let safe_title = title_with_ext.trim_end_matches(".html");

    if wiki_type != "wiki" {
        println!("   -> Skipping unknown wiki type: {}", wiki_type);
        return Ok(());
    }

    println!("Processing [{}] -> {}", wiki_type, safe_title);
    let raw_html = fs::read_to_string(path)?;

    // Aufruf unserer neuen sauberen Funktion mit durchgereichter Datenbank.
    let processed_data_bytes = article_processing::process_wikipedia_article(
        "QID_TEST",
        &raw_html,
        table,
        search_key_buffer,
        settings,
        lang,
    );

    let output_text = match std::str::from_utf8(&processed_data_bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            println!("   -> Warning: Output is not valid UTF-8. Saving as hex/binary summary.");
            format!("Binary data length: {} bytes", processed_data_bytes.len())
        }
    };

    let out_filename = format!("parsed_{}_{}_{}.txt", wiki_type, lang, safe_title);
    let out_path = out_dir.join(out_filename);

    fs::write(&out_path, output_text)?;
    println!("   -> Saved result to {:?}", out_path);

    Ok(())
}

// ============================================================================
// ÖFFENTLICHE HAUPTFUNKTIONEN
// ============================================================================

pub fn extract_sample_articles(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(&settings.paths.data_dir);
    let out_dir = PathBuf::from(&settings.paths.example_articles_dir);
    fs::create_dir_all(&out_dir)?;

    // Hier definieren wir die härtesten Wikipedia-Artikel (Edge-Cases) zum Testen
    let target_lang = "en";
    let target_titles = vec![
        "Fourier transform",         // Schweres LaTeX / Math
        "List of chemical elements", // Riesige Tabellen
        "Musical notation",          // Spezielle Tags (<score>)
        "Software Engineering",      // Tiefe Schachtelungen
    ];

    println!("Extraction of test articles started...");

    for wiki in &settings.database_content.wikis_to_include {
        if wiki != "wiki" {
            println!(
                "Skipping {}, only 'wiki' is supported for sample extraction.",
                wiki
            );
            continue;
        }

        let dir = data_dir.join(wiki);
        if !dir.exists() {
            println!("Directory for {} does not exist, skipping.", wiki);
            continue;
        }

        let pattern = &settings.match_patterns.wiki_zim_file_match_pattern;

        if let Some(zim_path) = find_zim_file(&dir, pattern, target_lang) {
            println!(
                "Searching in ZIM ({}_{}) for {:?}...",
                wiki, target_lang, target_titles
            );
            extract_from_zim(&zim_path, target_lang, &out_dir, &target_titles);
        } else {
            println!(
                "   -> No matching ZIM file found for {} and language '{}'",
                wiki, target_lang
            );
        }
    }

    println!("Finished extracting sample articles.");
    Ok(())
}

pub fn test_article_processing(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(&settings.paths.example_articles_dir);

    if !dir.exists() {
        println!(
            "Test directory {:?} does not exist. Please run the extraction first.",
            dir
        );
        return Ok(());
    }

    println!("Starting local parser tests...");

    // --- DATENBANK LADEN (Echt oder Dummy-In-Memory) ---
    let db = sitelinks_lookup::open_sitelinks_db(settings);
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(sitelinks_lookup::SITELINKS_TABLE)?;
    let mut search_key_buffer = String::with_capacity(256);
    // ---------------------------------------------------

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("html") {
                if let Err(e) =
                    test_single_article(&path, &dir, &table, &mut search_key_buffer, settings)
                {
                    println!("   -> Failed to test article {:?}: {}", path, e);
                }
            }
        }
    }

    println!("Finished parser tests. Check the .txt files in your example_articles directory.");
    Ok(())
}

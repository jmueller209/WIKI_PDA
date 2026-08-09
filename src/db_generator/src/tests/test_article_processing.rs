use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::utils::article_processing;
use crate::utils::settings::Settings;

pub fn extract_sample_articles(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(&settings.paths.data_dir);
    let out_dir = PathBuf::from(&settings.paths.example_articles_dir);
    fs::create_dir_all(&out_dir)?;

    struct TargetConfig {
        lang: &'static str,
        titles: Vec<&'static str>,
    }

    // ====================================================================
    // Define languages and sample articles here:
    // ====================================================================
    let mut targets: HashMap<&str, TargetConfig> = HashMap::new();

    targets.insert(
        "wiki",
        TargetConfig {
            lang: "en",
            // Wikipedia edge cases:
            // "Fourier transform": Heavy LaTeX/Math formulas (<math> tags).
            // "List of chemical elements": Massive, complex, sortable tables with symbols.
            // "Musical notation": Contains actual sheet music generated via <score> tags (LilyPond).
            titles: vec![
                "Fourier transform",
                "List of chemical elements",
                "Musical notation",
            ],
        },
    );

    targets.insert(
        "wiktionary",
        TargetConfig {
            lang: "en",
            // Wiktionary edge cases:
            // "set": One of the longest dictionary entries. Deeply nested ordered lists, multiple etymologies.
            // "go": Very complex verb conjugation tables and pronunciation (IPA/audio) templates.
            titles: vec!["house", "mother"],
        },
    );

    targets.insert(
        "wikiquote",
        TargetConfig {
            lang: "en",
            // Wikiquote edge cases:
            // "The Simpsons": Dialogue formatting, character tags, and season/episode headers.
            // "Albert Einstein": Standard bulleted quotes, sourced citations, and a "Misattributed" section.
            titles: vec!["The Simpsons", "Albert Einstein"],
        },
    );

    targets.insert(
        "wikivoyage",
        TargetConfig {
            lang: "en",
            // Wikivoyage edge cases:
            // "New York City": Heavy use of Geo-tags, interactive maps, and vCard/POI (Point of Interest) templates.
            // "Tokyo": Warning/Info boxes, very deep section nesting.
            titles: vec!["New York City", "London"],
        },
    );

    targets.insert(
        "wikisource",
        TargetConfig {
            lang: "en",
            // Wikisource edge cases:
            // "The Raven": <poem> tags, forced line breaks, and center alignment.
            // "The Tragedy of Hamlet, Prince of Denmark": Theatrical play formatting, character dialogue indentation.
            titles: vec!["The Raven", "Alice's Adventures in Wonderland"],
        },
    );

    targets.insert(
        "wikiversity",
        TargetConfig {
            lang: "en",
            // Wikiversity edge cases:
            // "Special Relativity": Heavy math, educational info-boxes, and embedded quizzes.
            // "Python": Syntax highlighting, code block tags (<source> / <syntaxhighlight>).
            titles: vec!["Software Engineering", "C++"],
        },
    );

    targets.insert(
        "wikibooks",
        TargetConfig {
            lang: "en",
            // Wikibooks edge cases:
            // "LaTeX": Book navigation templates (Next/Previous chapter), rich formatting.
            // "Cookbook:Chocolate Chip Cookies": Recipe templates, ingredient lists, step-by-step formatting.
            titles: vec!["Chess", "Cookbook:Chocolate Chip Cookies"],
        },
    );
    // ====================================================================

    println!("Extraction of test articles started...");

    for wiki in &settings.database_content.wikis_to_include {
        let target_config = match targets.get(wiki.as_str()) {
            Some(config) => config,
            None => {
                println!("No sample articles defined for {}, skipping.", wiki);
                continue;
            }
        };

        let dir = data_dir.join(wiki);
        if !dir.exists() {
            println!("Directory for {} does not exist, skipping.", wiki);
            continue;
        }

        let raw_pattern = match wiki.as_str() {
            "wiki" => &settings.match_patterns.wiki_zim_file_match_pattern,
            "wiktionary" => &settings.match_patterns.wiktionary_zim_file_match_pattern,
            "wikiquote" => &settings.match_patterns.wikiquote_zim_file_match_pattern,
            "wikisource" => &settings.match_patterns.wikisource_zim_file_match_pattern,
            "wikivoyage" => &settings.match_patterns.wikivoyage_zim_file_match_pattern,
            "wikiversity" => &settings.match_patterns.wikiversity_zim_file_match_pattern,
            "wikibooks" => &settings.match_patterns.wikibooks_zim_file_match_pattern,
            _ => continue,
        };

        let regex_str = raw_pattern.replace("{lang}", "(?P<lang>[a-zA-Z-]+)");
        let re = Regex::new(&regex_str).expect("Invalid Regex Pattern in config");

        let mut found_zim_path = None;

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                if let Some(captures) = re.captures(&file_name) {
                    if let Some(lang_match) = captures.name("lang") {
                        if lang_match.as_str() == target_config.lang {
                            found_zim_path = Some(path);
                            break;
                        }
                    }
                }
            }
        }

        if let Some(zim_path) = found_zim_path {
            println!(
                "Searching in ZIM ({}_{}) for {:?}...",
                wiki, target_config.lang, target_config.titles
            );

            let zim_file = match zim::Zim::new(&zim_path) {
                Ok(z) => z,
                Err(_) => continue,
            };

            let mut remaining_targets = target_config.titles.clone();

            for direntry_result in zim_file.iterate_by_urls() {
                if remaining_targets.is_empty() {
                    break;
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

                let decoded_url;
                let raw_string = if !direntry.title.is_empty() {
                    direntry.title.as_str()
                } else {
                    decoded_url = urlencoding::decode(&direntry.url)
                        .unwrap_or(std::borrow::Cow::Borrowed(&direntry.url));
                    &decoded_url
                };

                let clean_title = raw_string.replace('_', " ");

                if let Some(pos) = remaining_targets
                    .iter()
                    .position(|&t| clean_title.eq_ignore_ascii_case(t))
                {
                    if let Ok(Some(content)) = zim_file.entry_content(&direntry) {
                        let raw_text = content
                            .with(|bytes| unsafe {
                                std::str::from_utf8_unchecked(bytes).to_string()
                            })
                            .unwrap_or_default();

                        let safe_title = remaining_targets[pos].replace(['/', '\\', ' '], "_");
                        let out_filename =
                            format!("sample_{}_{}_{}.html", wiki, target_config.lang, safe_title);
                        let out_path = out_dir.join(&out_filename);

                        if let Err(e) = fs::write(&out_path, raw_text) {
                            println!("   -> Failed to save {:?}: {}", out_path, e);
                        } else {
                            println!("   -> Saved raw article: {:?}", out_path);
                        }

                        remaining_targets.remove(pos);
                    }
                }
            }
            if !remaining_targets.is_empty() {
                println!(
                    "   -> Warning: Could not find these articles: {:?}",
                    remaining_targets
                );
            }
        } else {
            println!(
                "   -> No matching ZIM file found for {} and language '{}'",
                wiki, target_config.lang
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

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("html") {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let parts: Vec<&str> = filename.splitn(4, '_').collect();
                if parts.len() < 4 || parts[0] != "sample" {
                    continue;
                }

                let wiki_type = parts[1];
                let _lang = parts[2];
                let title_with_ext = parts[3];
                let safe_title = title_with_ext.trim_end_matches(".html");

                println!("Processing [{}] -> {}", wiki_type, safe_title);
                let raw_html = fs::read_to_string(&path)?;

                let processed_data_bytes: Vec<u8> = match wiki_type {
                    "wiki" => article_processing::process_wiki(&raw_html).into_bytes(),
                    "wiktionary" => article_processing::process_wiktionary(&raw_html).into_bytes(),
                    "wikiquote" => article_processing::process_wikiquote(&raw_html).into_bytes(),
                    "wikisource" => article_processing::process_wikisource(&raw_html).into_bytes(),
                    "wikivoyage" => article_processing::process_wikivoyage(&raw_html).into_bytes(),
                    "wikibooks" => article_processing::process_wikibooks(&raw_html).into_bytes(),
                    "wikiversity" => {
                        article_processing::process_wikiversity(&raw_html).into_bytes()
                    }
                    _ => {
                        println!("   -> Unknown wiki type: {}, skipping.", wiki_type);
                        continue;
                    }
                };

                let output_text = match std::str::from_utf8(&processed_data_bytes) {
                    Ok(text) => text.to_string(),
                    Err(_) => {
                        println!(
                            "   -> Warning: Output is not valid UTF-8. Saving as hex/binary summary."
                        );
                        format!("Binary data length: {} bytes", processed_data_bytes.len())
                    }
                };

                let out_filename = format!("parsed_{}_{}_{}.txt", wiki_type, _lang, safe_title);
                let out_path = dir.join(out_filename);
                fs::write(&out_path, output_text)?;
                println!("   -> Saved result to {:?}", out_path);
            }
        }
    }

    println!("Finished parser tests. Check the .txt files in your example_articles directory.");
    Ok(())
}

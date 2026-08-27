use html2text::from_read;
use kuchikiki::traits::*;
use redb::ReadOnlyTable;
// use std::panic::catch_unwind;

use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;

fn clean_html_tree(
    raw_html: &str,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
) -> String {
    let document = kuchikiki::parse_html().one(raw_html);

    let content_node = match document.select_first("div.mw-parser-output") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document.clone(),
    };

    let selectors_to_remove = [
        // --- WE ARE KEEPING TABLES NOW (removed "table") ---

        // --- META, NAVIGATION & HATNOTES ---
        "div.navbox",         // Bottom navigation boxes
        "div.metadata",       // Meta warnings
        "div.printfooter",    // Print info
        "div.mw-editsection", // "Edit" links
        "div.hatnote",        // "See also", "Main article", "Not to be confused with"
        "div.rellink",        // Alternative cross-reference links
        "dl.rellink",         // Sometimes used for cross-references
        // --- SOURCES & FOOTNOTES ---
        "sup.reference", // Standard Wikipedia footnotes
        "sup.mw-ref",    // ZIM / Parsoid footnotes
        ".mw-ref",       // Fallback for other footnote tags
        "sup.noprint",   // Often used for [note 1] or [citation needed]
        "div.reflist",   // The entire references block at the bottom
        "ol.references", // The references list itself
        // --- MATH CLEANUP (Prevents triple duplication) ---
        "span.mwe-math-mathml-a11y", // Hidden MathML for screen readers
        "math",                      // MathML tags
        "annotation",                // Raw TeX annotations
    ];

    for selector in selectors_to_remove.iter() {
        if let Ok(elements) = content_node.select(selector) {
            for element in elements {
                element.as_node().detach();
            }
        }
    }

    if let Ok(a_tags) = content_node.select("a") {
        for a_node in a_tags {
            let mut attrs = a_node.attributes.borrow_mut();

            if let Some(href) = attrs.get("href") {
                let is_external =
                    href.starts_with("http") || href.starts_with("//") || href.starts_with("www.");
                let is_anchor = href.starts_with('#');

                let title_attr = attrs.get("title").map(|s| s.to_string());
                let href_val = href.to_string();

                attrs.remove("href");
                drop(attrs);

                if !is_external && !is_anchor {
                    let target_title = title_attr.unwrap_or_default();

                    if !target_title.is_empty() {
                        let (qid_opt, _) = sitelinks_lookup::lookup_qid_from_sitelinks(
                            table,
                            search_key_buffer,
                            settings,
                            lang,
                            &target_title,
                            &href_val,
                        );

                        let qid_str = qid_opt.unwrap_or_else(|| "NOT_FOUND".to_string());

                        let node_ref = a_node.as_node();
                        node_ref.insert_before(kuchikiki::NodeRef::new_text("["));
                        node_ref
                            .insert_after(kuchikiki::NodeRef::new_text(format!("][#{}]", qid_str)));
                    }
                }
            }
        }
    }

    let mut cleaned_html = Vec::new();
    let _ = content_node.serialize(&mut cleaned_html);
    String::from_utf8_lossy(&cleaned_html).to_string()
}

fn convert_html_to_plain_text(cleaned_html: &str) -> String {
    from_read(cleaned_html.as_bytes(), 100).unwrap_or_default()
}

pub fn process_wikipedia_article(
    qid: &str,
    raw_html: &str,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
) -> Vec<u8> {
    let cleaned_html = clean_html_tree(raw_html, table, search_key_buffer, settings, lang);

    let plain_text = convert_html_to_plain_text(&cleaned_html);

    let formatted_output = format!("--- QID: {} ---\n\n{}\n\n", qid, plain_text);

    formatted_output.into_bytes()
}

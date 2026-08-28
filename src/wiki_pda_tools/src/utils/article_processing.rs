use html2text::from_read;
use kuchikiki::traits::*;
use redb::ReadOnlyTable;

use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;

pub trait ArticleProcessor: Send + Sync {
    fn process(
        &self,
        qid: &str,
        raw_html: &str,
        table: &ReadOnlyTable<&str, &str>,
        search_key_buffer: &mut String,
        settings: &Settings,
        lang: &str,
    ) -> Result<Vec<u8>, String>;
}

pub struct DefaultArticleProcessor;

impl ArticleProcessor for DefaultArticleProcessor {
    fn process(
        &self,
        qid: &str,
        raw_html: &str,
        table: &ReadOnlyTable<&str, &str>,
        search_key_buffer: &mut String,
        settings: &Settings,
        lang: &str,
    ) -> Result<Vec<u8>, String> {
        let cleaned_html = clean_html_tree(raw_html, table, search_key_buffer, settings, lang)?;

        let plain_text = convert_html_to_plain_text(&cleaned_html)?;

        let formatted_output = format!("--- QID: {} ---\n\n{}\n\n", qid, plain_text);

        Ok(formatted_output.into_bytes())
    }
}

fn clean_html_tree(
    raw_html: &str,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
) -> Result<String, String> {
    let document = kuchikiki::parse_html().one(raw_html);

    let content_node = match document.select_first("div.mw-parser-output") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document.clone(),
    };

    let selectors_to_remove = [
        "div.navbox",
        "div.metadata",
        "div.printfooter",
        "div.mw-editsection",
        "div.hatnote",
        "div.rellink",
        "dl.rellink",
        "sup.reference",
        "sup.mw-ref",
        ".mw-ref",
        "sup.noprint",
        "div.reflist",
        "ol.references",
        "span.mwe-math-mathml-a11y",
        "math",
        "annotation",
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
            let mut attrs = match a_node.attributes.try_borrow_mut() {
                Ok(a) => a,
                Err(_) => continue,
            };

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

                        if node_ref.parent().is_some() {
                            node_ref.insert_before(kuchikiki::NodeRef::new_text("["));
                            node_ref.insert_after(kuchikiki::NodeRef::new_text(format!(
                                "][#{}]",
                                qid_str
                            )));
                        }
                    }
                }
            }
        }
    }

    let mut cleaned_html = Vec::new();
    content_node
        .serialize(&mut cleaned_html)
        .map_err(|e| format!("Failed to serialize HTML tree: {}", e))?;

    Ok(String::from_utf8_lossy(&cleaned_html).to_string())
}

fn convert_html_to_plain_text(cleaned_html: &str) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| html2text::from_read(cleaned_html.as_bytes(), 10000));

    match result {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(parse_error)) => Err(format!("html2text parsing error: {}", parse_error)),
        Err(panic_payload) => {
            let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown internal panic".to_string()
            };
            Err(format!("html2text library crashed! Reason: {}", panic_msg))
        }
    }
}

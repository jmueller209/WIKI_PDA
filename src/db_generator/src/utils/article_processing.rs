use html2text::from_read;
use kuchikiki::traits::*;
use std::panic::catch_unwind;

pub fn process_wiki(raw_html: &str) -> String {
    let document = kuchikiki::parse_html().one(raw_html);

    let content_node = match document.select_first("div.mw-parser-output") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document.clone(),
    };


    let selectors_to_remove = [
        "table",              
        "div.navbox",         
        "div.metadata",       
        "div.printfooter",    
        "div.mw-editsection", 
        "sup.reference",     
    ];

    for selector in selectors_to_remove.iter() {
        if let Ok(elements) = content_node.select(selector) {
            for element in elements {
                element.as_node().detach();
            }
        }
    }

    let mut cleaned_html = Vec::new();
    let _ = content_node.serialize(&mut cleaned_html);
    let cleaned_html_string = String::from_utf8_lossy(&cleaned_html);

    let plain_text = catch_unwind(|| from_read(cleaned_html_string.as_bytes(), 100))
        .unwrap_or_else(|_| Ok(cleaned_html_string.to_string()))
        .unwrap_or_else(|_| cleaned_html_string.to_string());

    plain_text
}

pub fn process_wiktionary(raw_html: &str) -> String {
    process_wiki(raw_html)
}

pub fn process_wikiquote(raw_html: &str) -> String {
    process_wiki(raw_html)
}

pub fn process_wikisource(raw_html: &str) -> String {
    process_wiki(raw_html)
}

pub fn process_wikivoyage(raw_html: &str) -> String {
    process_wiki(raw_html)
}

pub fn process_wikiversity(raw_html: &str) -> String {
    process_wiki(raw_html)
}

pub fn process_wikibooks(raw_html: &str) -> String {
    process_wiki(raw_html)
}


// Wrapper function: Do NOT change this!
pub fn process_article(article_kind: &str, qid: &str, article_string: &str) -> Vec<u8> {
    let plain_text = match article_kind {
        "wiki" => process_wiki(article_string),
        "wiktionary" => process_wiktionary(article_string),
        "wikiquote" => process_wikiquote(article_string),
        "wikisource" => process_wikisource(article_string),
        "wikivoyage" => process_wikivoyage(article_string),
        "wikiversity" => process_wikiversity(article_string),
        "wikibooks" => process_wikibooks(article_string),
        w => panic!("{w} is not a valid wiki"),
    };

    let formatted_output = format!(
        "--- WIKI KIND: {} | QID: {} ---\n\n{}\n\n",
        article_kind, qid, plain_text
    );

    formatted_output.into_bytes()
}

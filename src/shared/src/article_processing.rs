use html2text::from_read;
use kuchikiki::traits::*;
use std::panic::catch_unwind;

pub fn process_article(article_kind: &str, qid: &str, article_string: &str) -> Vec<u8> {
    // 1. HTML in einen DOM-Baum parsen (wie BeautifulSoup)
    let document = kuchikiki::parse_html().one(article_string);

    // 2. Den "mw-parser-output" div finden (den Hauptinhalt)
    // Wenn wir ihn nicht finden, arbeiten wir mit dem ganzen Dokument weiter
    let content_node = match document.select_first("div.mw-parser-output") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document.clone(),
    };

    // 3. CHIRURGISCHE ENTFERNUNG (wie div.decompose() in Python)
    // Wir löschen alle Tabellen, Infoboxen und Referenzen ([1], [2])
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
                // Das ist exakt table.decompose()
                element.as_node().detach();
            }
        }
    }

    // 4. Das bereinigte HTML wieder in einen String verwandeln
    let mut cleaned_html = Vec::new();
    let _ = content_node.serialize(&mut cleaned_html);
    let cleaned_html_string = String::from_utf8_lossy(&cleaned_html);

    // 5. In sauberen Text umwandeln (TooNarrow passiert jetzt extrem selten,
    // da wir die Tabellen entfernt haben, wir sichern es trotzdem ab)
    let plain_text = catch_unwind(|| from_read(cleaned_html_string.as_bytes(), 100))
        .unwrap_or_else(|_| cleaned_html_string.to_string());

    // 6. Formatieren und als Bytes zurückgeben
    let formatted_output = format!(
        "--- WIKI KIND: {} | QID: {} ---\n\n{}\n\n",
        article_kind, qid, plain_text
    );

    formatted_output.into_bytes()
}

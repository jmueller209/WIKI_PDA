use html2text;
use kuchikiki::traits::*;
use redb::ReadOnlyTable;

use crate::utils::settings::Settings;
use crate::utils::sitelinks_lookup;

// ============================================================================
// Public interface
// ============================================================================

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
        let (cleaned_html, saved_tables) =
            clean_html_tree(raw_html, table, search_key_buffer, settings, lang)?;

        let mut plain_text = convert_html_to_plain_text(&cleaned_html)?;

        inject_tables(&mut plain_text, saved_tables);

        let output = format!("--- QID: {} ---\n\n{}\n\n", qid, plain_text);

        Ok(output.into_bytes())
    }
}

// ============================================================================
// Table representation
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableRowKind {
    Header,
    Data,
}

impl TableRowKind {
    fn marker(self) -> &'static str {
        match self {
            Self::Header => "H",
            Self::Data => "R",
        }
    }
}

#[derive(Debug)]
struct TableRow {
    kind: TableRowKind,
    cells: Vec<String>,
}

#[derive(Debug, Default)]
struct ProcessedTable {
    rows: Vec<TableRow>,
}

impl ProcessedTable {
    fn serialize(&self) -> String {
        let mut output = String::new();

        for row in &self.rows {
            output.push_str(row.kind.marker());
            output.push('|');
            output.push_str(&row.cells.join("|"));
            output.push('\n');
        }

        output
    }
}

// ============================================================================
// HTML cleaning
// ============================================================================

fn clean_html_tree(
    raw_html: &str,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
) -> Result<(Vec<u8>, Vec<String>), String> {
    let document = kuchikiki::parse_html().one(raw_html);

    let content_node = match document.select_first("div.mw-parser-output") {
        Ok(node) => node.as_node().clone(),
        Err(_) => document,
    };

    remove_unwanted_elements(&content_node);

    convert_internal_links(&content_node, table, search_key_buffer, settings, lang);

    process_math(&content_node);

    let saved_tables = extract_tables(&content_node);

    let mut cleaned_html = Vec::new();

    content_node
        .serialize(&mut cleaned_html)
        .map_err(|error| format!("Failed to serialize HTML tree: {}", error))?;

    Ok((cleaned_html, saved_tables))
}

// ============================================================================
// Unwanted HTML
// ============================================================================

const SELECTORS_TO_REMOVE: &str = concat!(
    "style,",
    "script,",
    "link,",
    ".navbox,",
    "div.vertical-navbox,",
    "div.metadata,",
    "div.printfooter,",
    "div.mw-editsection,",
    ".sidebar,",
    "div.hatnote,",
    "div.rellink,",
    "dl.rellink,",
    "sup.reference,",
    "sup.mw-ref,",
    "span.mw-ref,",
    ".mw-ref,",
    ".noprint,",
    "div.reflist,",
    "ol.references,",
    "div.refbegin,",
    "div.refbegin-columns,",
    "table.infobox,",
    "table.ambox,",
    "table.tmbox,",
    "table.cmbox,",
    "table.fmbox,",
    "table.ombox,",
    "div.ambox,",
    "div.tmbox,",
    "div.cmbox,",
    "div.fmbox,",
    "div.ombox,",
    "div#toc,",
    "div.toc,",
    "#coordinates,",
    "span.coordinates,",
    ".shortdescription,",
    "div.topicon,",
    "figure,",
    "div.thumb,",
    ".gallery,",
    ".mw-file-description,",
    ".mw-empty-elt,",
    ".mw-cite-backlink,",
    ".mw-references-wrap,",
    ".sistersitebox,",
    ".side-box,",
    ".zim-footer,",
    "div#catlinks",
);

fn remove_unwanted_elements(content_node: &kuchikiki::NodeRef) {
    let Ok(elements) = content_node.select(SELECTORS_TO_REMOVE) else {
        return;
    };

    // Collect first because detaching nodes while iterating the selector
    // can invalidate the iterator.
    let nodes_to_remove: Vec<_> = elements.map(|element| element.as_node().clone()).collect();

    for node in nodes_to_remove {
        node.detach();
    }
}

// ============================================================================
// Internal Wikipedia links
// ============================================================================

fn convert_internal_links(
    node: &kuchikiki::NodeRef,
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
) {
    let Ok(links) = node.select("a") else {
        return;
    };

    let links: Vec<_> = links.collect();

    for link in links {
        let (href, title) = match link.attributes.try_borrow() {
            Ok(attributes) => {
                let href = attributes.get("href").map(str::to_owned);
                let title = attributes
                    .get("title")
                    .map(str::to_owned)
                    .unwrap_or_default();

                (href, title)
            }

            Err(_) => continue,
        };

        let Some(href) = href else {
            continue;
        };

        let link_type = classify_link(&href);

        if matches!(link_type, LinkType::Internal) && !title.is_empty() {
            let qid = sitelinks_lookup::lookup_qid_from_sitelinks(
                table,
                search_key_buffer,
                settings,
                lang,
                &title,
                &href,
            )
            .0
            .unwrap_or_else(|| "Q0".to_string());

            let qid_node = kuchikiki::NodeRef::new_text(format!(" [{}]", qid));

            link.as_node().append(qid_node);
        }

        remove_link_attributes(&link);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkType {
    Internal,
    External,
    Fragment,
}

fn classify_link(href: &str) -> LinkType {
    if href.starts_with('#') {
        LinkType::Fragment
    } else if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("//")
        || href.starts_with("www.")
    {
        LinkType::External
    } else {
        LinkType::Internal
    }
}

fn remove_link_attributes(link: &kuchikiki::NodeDataRef<kuchikiki::ElementData>) {
    if let Ok(mut attributes) = link.attributes.try_borrow_mut() {
        attributes.remove("href");
        attributes.remove("title");
    }
}

// ============================================================================
// Math
// ============================================================================

fn process_math(content_node: &kuchikiki::NodeRef) {
    let Ok(math_elements) = content_node.select("span.mwe-math-element, math") else {
        return;
    };

    let math_nodes: Vec<_> = math_elements
        .map(|element| element.as_node().clone())
        .collect();

    for math_node in math_nodes {
        let Some(latex) = extract_latex(&math_node) else {
            math_node.detach();
            continue;
        };

        let replacement = kuchikiki::NodeRef::new_text(format!(" ${}$ ", latex));

        math_node.insert_after(replacement);
        math_node.detach();
    }
}

fn extract_latex(node: &kuchikiki::NodeRef) -> Option<String> {
    let mut annotations = node
        .select("annotation[encoding='application/x-tex']")
        .ok()?;

    let annotation = annotations.next()?;

    let mut latex = annotation.text_contents().trim().to_string();

    latex = remove_math_wrapper(latex);

    Some(latex)
}

fn remove_math_wrapper(mut latex: String) -> String {
    const DISPLAY_PREFIX: &str = r"{\displaystyle ";
    const TEXT_PREFIX: &str = r"{\textstyle ";

    if latex.starts_with(DISPLAY_PREFIX) {
        latex = latex[DISPLAY_PREFIX.len()..].to_string();

        if latex.ends_with('}') {
            latex.pop();
        }
    } else if latex.starts_with(TEXT_PREFIX) {
        latex = latex[TEXT_PREFIX.len()..].to_string();

        if latex.ends_with('}') {
            latex.pop();
        }
    }

    latex.trim().to_string()
}

// ============================================================================
// Table extraction
// ============================================================================

fn extract_tables(content_node: &kuchikiki::NodeRef) -> Vec<String> {
    let Ok(tables) = content_node.select("table.wikitable") else {
        return Vec::new();
    };

    let table_nodes: Vec<_> = tables.map(|element| element.as_node().clone()).collect();

    let mut serialized_tables = Vec::with_capacity(table_nodes.len());

    for (index, table_node) in table_nodes.into_iter().enumerate() {
        let table = parse_table(&table_node);

        serialized_tables.push(table.serialize());

        replace_table_with_placeholder(&table_node, index);
    }

    serialized_tables
}

fn parse_table(table_node: &kuchikiki::NodeRef) -> ProcessedTable {
    let Ok(rows) = table_node.select("tr") else {
        return ProcessedTable::default();
    };

    let mut table = ProcessedTable { rows: Vec::new() };

    for row in rows {
        if let Some(parsed_row) = parse_table_row(&row.as_node()) {
            table.rows.push(parsed_row);
        }
    }

    table
}

fn parse_table_row(row_node: &kuchikiki::NodeRef) -> Option<TableRow> {
    let header_count = row_node
        .select("th")
        .map(|cells| cells.count())
        .unwrap_or(0);

    let data_count = row_node
        .select("td")
        .map(|cells| cells.count())
        .unwrap_or(0);

    if header_count == 0 && data_count == 0 {
        return None;
    }

    let kind = if header_count > 0 && data_count == 0 {
        TableRowKind::Header
    } else {
        TableRowKind::Data
    };

    let cells = extract_table_cells(row_node);

    if cells.is_empty() {
        return None;
    }

    Some(TableRow { kind, cells })
}

fn extract_table_cells(row_node: &kuchikiki::NodeRef) -> Vec<String> {
    let Ok(cells) = row_node.select("th, td") else {
        return Vec::new();
    };

    cells
        .map(|cell| clean_table_cell(cell.as_node().text_contents()))
        .collect()
}

fn clean_table_cell(mut text: String) -> String {
    remove_uniq_markers(&mut text);

    text.replace('\u{2060}', "")
        .replace('\n', " ")
        .replace('|', "¦")
        .trim()
        .to_string()
}

fn remove_uniq_markers(text: &mut String) {
    const PREFIX: &str = "\x7F'\"`UNIQ--";
    const SUFFIX: &str = "`\"'\x7F";

    while let Some(start) = text.find(PREFIX) {
        let search_start = start + PREFIX.len();

        let Some(relative_end) = text[search_start..].find(SUFFIX) else {
            break;
        };

        let end = search_start + relative_end + SUFFIX.len();

        text.replace_range(start..end, "");
    }
}

fn replace_table_with_placeholder(table_node: &kuchikiki::NodeRef, index: usize) {
    let placeholder = format!("\n\nWIKIPEDIA_TABLE_{}\n\n", index);

    table_node.insert_after(kuchikiki::NodeRef::new_text(placeholder));
    table_node.detach();
}

// ============================================================================
// HTML -> plain text
// ============================================================================

fn convert_html_to_plain_text(cleaned_html: &[u8]) -> Result<String, String> {
    const TEXT_WIDTH: usize = 10_000;

    let result = std::panic::catch_unwind(|| html2text::from_read(cleaned_html, TEXT_WIDTH));

    match result {
        Ok(Ok(text)) => Ok(normalize_text(text)),

        Ok(Err(error)) => Err(format!("html2text parsing error: {}", error)),

        Err(payload) => Err(format!(
            "html2text library crashed! Reason: {}",
            panic_message(payload)
        )),
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "Unknown internal panic".to_string()
    }
}

// ============================================================================
// Table injection
// ============================================================================

fn inject_tables(plain_text: &mut String, tables: Vec<String>) {
    for (index, table) in tables.into_iter().enumerate() {
        let placeholder = format!("WIKIPEDIA_TABLE_{}", index);

        let replacement = format!("[[TABLE_START]]\n{}\n[[TABLE_END]]", table.trim_end());

        *plain_text = plain_text.replace(&placeholder, &replacement);
    }
}

// ============================================================================
// Text normalization
// ============================================================================

const UNWANTED_SECTIONS: &[&str] = &[
    "## Further reading",
    "## External links",
    "## References",
    "## Bibliography",
    "## Notes",
];

fn normalize_text(text: String) -> String {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    let keep = determine_kept_lines(&lines);

    build_normalized_text(&lines, &keep)
}

fn determine_kept_lines(lines: &[&str]) -> Vec<bool> {
    let mut keep = vec![true; lines.len()];
    let mut in_unwanted_section = false;

    for index in 0..lines.len() {
        let line = lines[index].trim_start();

        update_section_state(line, &mut in_unwanted_section);

        if in_unwanted_section {
            keep[index] = false;
            continue;
        }

        if line.starts_with('#') {
            keep[index] = section_has_content(lines, index);
        }
    }

    keep
}

fn update_section_state(line: &str, in_unwanted_section: &mut bool) {
    if line.starts_with("## ") {
        *in_unwanted_section = UNWANTED_SECTIONS
            .iter()
            .any(|section| line.starts_with(section));
    } else if line.starts_with('#') {
        *in_unwanted_section = false;
    }
}

fn section_has_content(lines: &[&str], heading_index: usize) -> bool {
    for line in &lines[(heading_index + 1)..] {
        let line = line.trim();

        if line.starts_with('#') {
            return false;
        }

        if !line.is_empty() {
            return true;
        }
    }

    false
}

fn build_normalized_text(lines: &[&str], keep: &[bool]) -> String {
    let mut output = String::with_capacity(lines.iter().map(|line| line.len() + 1).sum());

    let mut previous_blank = false;

    for (index, original_line) in lines.iter().enumerate() {
        if !keep[index] {
            continue;
        }

        let mut line = original_line.trim_end().to_string();

        if line.trim().is_empty() {
            if !previous_blank {
                output.push('\n');
                previous_blank = true;
            }

            continue;
        }

        line = clean_long_border(line);
        line = collapse_large_spaces(&line);

        output.push_str(&line);
        output.push('\n');

        previous_blank = false;
    }

    if output.ends_with('\n') {
        output.pop();
    }

    output
}

// ============================================================================
// Formatting helpers
// ============================================================================

fn clean_long_border(mut line: String) -> String {
    if line.chars().count() <= 100 {
        return line;
    }

    let is_border = line.chars().all(is_border_character);

    if is_border {
        line = line.chars().take(80).collect();
    }

    line
}

fn is_border_character(character: char) -> bool {
    matches!(
        character,
        '-' | '_'
            | '='
            | '*'
            | '+'
            | ' '
            | '─'
            | '│'
            | '┌'
            | '┬'
            | '┐'
            | '├'
            | '┼'
            | '┤'
            | '└'
            | '┴'
            | '┘'
    )
}

fn collapse_large_spaces(line: &str) -> String {
    let mut output = String::with_capacity(line.len());

    let mut space_count = 0;

    for character in line.chars() {
        if character == ' ' {
            space_count += 1;
            continue;
        }

        append_spaces(&mut output, space_count);
        output.push(character);

        space_count = 0;
    }

    append_spaces(&mut output, space_count);

    output
}

fn append_spaces(output: &mut String, count: usize) {
    if count > 10 {
        output.push_str("   ");
    } else if count > 0 {
        output.push_str(&" ".repeat(count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use redb::backends::InMemoryBackend;
    use redb::{Database, TableDefinition};

    const MOCK_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sitelinks");

    fn run_parser_test(test_name: &str, raw_html: &str) {
        let db = Database::builder()
            .create_with_backend(InMemoryBackend::new())
            .unwrap();

        let write_txn = db.begin_write().unwrap();
        {
            let _table = write_txn.open_table(MOCK_TABLE).unwrap();
        }
        write_txn.commit().unwrap();

        let read_txn = db.begin_read().unwrap();
        let table = read_txn.open_table(MOCK_TABLE).unwrap();
        let mut search_buffer = String::new();
        let settings = Settings::default();

        let (cleaned_html, saved_tables) =
            clean_html_tree(raw_html, &table, &mut search_buffer, &settings, "en").unwrap();

        let mut plain_text = convert_html_to_plain_text(&cleaned_html).unwrap();

        for (index, table_csv) in saved_tables.into_iter().enumerate() {
            let placeholder = format!("WIKIPEDIA_TABLE_{}", index);
            let injection = format!(
                "\n\n[[TABLE_START]]\n{}\n[[TABLE_END]]\n\n",
                table_csv.trim()
            );
            plain_text = plain_text.replace(&placeholder, &injection);
        }

        assert_snapshot!(test_name, plain_text);
    }

    #[test]
    fn test_math_extraction() {
        let html = r#"
        <div class="mw-parser-output">
            <p>The famous equation <span class="mwe-math-element">
            <math><annotation encoding="application/x-tex">E=mc^2</annotation></math>
            </span> changed physics.</p>
        </div>"#;
        run_parser_test("math_extraction", html);
    }

    #[test]
    fn test_internal_link_appends_q0() {
        let html = r###"
        <div class="mw-parser-output">
            <p>Albert Einstein was a <a href="/wiki/Theoretical_physics" title="Theoretical physics">theoretical physicist</a>.</p>
        </div>"###;
        run_parser_test("internal_link_appends_q0", html);
    }

    #[test]
    fn test_table_generation() {
        let html = r###"
        <div class="mw-parser-output">
            <p>Here is some data:</p>
            <table class="wikitable">
                <tr>
                    <th>Column A</th>
                    <th>Column B | Data</th>
                </tr>
                <tr>
                    <td>Row 1</td>
                    <td>Value 1&#x7F;'"`UNIQ--ref-00000000-QINU`"'&#x7F;</td>
                </tr>
            </table>
        </div>"###;
        run_parser_test("table_generation", html);
    }

    #[test]
    fn test_removes_junk_elements() {
        let html = r###"
        <div class="mw-parser-output">
            <table class="infobox"><tr><td>This should be deleted</td></tr></table>
            <style>body { background: black; }</style>
            <script>alert("Delete me");</script>
            <div class="navbox">Also delete this</div>
            <p>This is the only valid text.</p>
            <sup class="reference">[1]</sup>
        </div>"###;
        run_parser_test("removes_junk_elements", html);
    }

    #[test]
    fn test_removes_unwanted_sections() {
        let html = r###"
        <div class="mw-parser-output">
            <h2>History</h2>
            <p>This is good content.</p>
            <h2>References</h2>
            <p>1. Some book</p>
            <p>2. Some link</p>
            <h2>External links</h2>
            <ul><li>Link 1</li></ul>
        </div>"###;
        run_parser_test("removes_unwanted_sections", html);
    }

    #[test]
    fn test_normalize_text_spacing_and_borders() {
        let input = "Column A                                                  Column B\n----------------------------------------------------------------------------------------------------------------------------------";
        let output = normalize_text(input.to_string());

        assert!(output.contains("Column A   Column B"));
        assert!(!output.contains("----------------------------------------------------------------------------------------------------------------------------------"));
        assert!(output.contains(&"-".repeat(80)));
    }
}

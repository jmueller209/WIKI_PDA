use redb::{Database, ReadOnlyTable, TableDefinition};
use std::borrow::Cow;
use std::path::PathBuf;

use crate::utils::constants;
use crate::utils::settings::Settings;

pub const SITELINKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sitelinks");

pub fn open_sitelinks_db(settings: &Settings) -> Database {
    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    let db_path = tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB);

    if db_path.exists() {
        return Database::open(&db_path).expect("Failed to open sitelink database");
    }

    println!("\nWARNING: Real sitelinks database not found!");
    println!("WARNING: Creating a temporary IN-MEMORY dummy database for testing.\n");

    let db = Database::builder()
        .create_with_backend(redb::backends::InMemoryBackend::new())
        .expect("Failed to create in-memory dummy database");

    let write_txn = db.begin_write().expect("Failed to start write transaction");
    write_txn
        .open_table(SITELINKS_TABLE)
        .expect("Failed to create table in dummy DB");
    write_txn.commit().expect("Failed to commit dummy DB setup");

    db
}

pub fn lookup_qid_from_sitelinks(
    table: &ReadOnlyTable<&str, &str>,
    search_key_buffer: &mut String,
    settings: &Settings,
    lang: &str,
    direntry_title: &str,
    direntry_url: &str,
) -> (Option<String>, String) {
    let text_delim = &settings.other.text_delimiter;

    let decoded_url;
    let raw_title = if !direntry_title.is_empty() {
        direntry_title
    } else {
        decoded_url = urlencoding::decode(direntry_url).unwrap_or(Cow::Borrowed(direntry_url));
        &decoded_url
    };

    let primary_title = if raw_title.contains('_') {
        raw_title.replace('_', " ").trim().to_string()
    } else {
        raw_title.trim().to_string()
    };

    search_key_buffer.clear();
    search_key_buffer.push_str(lang);
    search_key_buffer.push_str(text_delim);
    search_key_buffer.push_str(&primary_title);

    if let Ok(Some(q)) = table.get(search_key_buffer.as_str()) {
        return (Some(q.value().to_string()), primary_title);
    }

    let decoded_url_fb = urlencoding::decode(direntry_url).unwrap_or(Cow::Borrowed(direntry_url));
    let mut fallback_title = if decoded_url_fb.contains('_') {
        decoded_url_fb.replace('_', " ").trim().to_string()
    } else {
        decoded_url_fb.trim().to_string()
    };

    if let Some(first_char) = fallback_title.chars().next() {
        if first_char.is_lowercase() {
            let mut chars = fallback_title.chars();
            if let Some(f) = chars.next() {
                fallback_title = f.to_uppercase().collect::<String>() + chars.as_str();
            }
        }
    }

    search_key_buffer.clear();
    search_key_buffer.push_str(lang);
    search_key_buffer.push_str(text_delim);
    search_key_buffer.push_str(&fallback_title);

    let result = if let Ok(Some(q)) = table.get(search_key_buffer.as_str()) {
        Some(q.value().to_string())
    } else {
        None
    };

    (result, primary_title)
}

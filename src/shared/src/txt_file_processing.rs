use rusqlite::{Connection, Result};
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::process::Command;

// Building an SQLite databse for sitelinks lookup
pub fn build_sitelink_database(txt_path: &str, db_path: &str, delimiter: &str) -> Result<()> {
    println!("Creating SQLite database from {}...", txt_path);
    let mut conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = OFF;
         PRAGMA synchronous = 0;
         PRAGMA cache_size = 1000000;
         PRAGMA locking_mode = EXCLUSIVE;
         PRAGMA temp_store = MEMORY;",
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sitelinks (
            lang TEXT NOT NULL,
            wiki TEXT NOT NULL,
            title TEXT NOT NULL,
            qid TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute("DELETE FROM sitelinks", [])?;
    let file = File::open(txt_path).expect("Could not open the file");
    let reader = io::BufReader::new(file);
    let tx = conn.transaction()?;

    {
        let mut stmt =
            tx.prepare("INSERT INTO sitelinks (lang, wiki, title, qid) VALUES (?1, ?2, ?3, ?4)")?;
        let mut count = 0;
        for line in reader.lines() {
            let line = line.expect("Could not read line");
            let parts: Vec<&str> = line.split(delimiter).collect();
            if parts.len() == 4 {
                let lang = parts[0].trim();
                let wiki = parts[1].trim();
                let title = parts[2].trim();
                let qid = parts[3].trim();

                stmt.execute([lang, wiki, title, qid])?;
                count += 1;
                if count % 500_000 == 0 {
                    println!("  ... {} entries copied", count);
                }
            }
        }
        println!("Imported a total of {} entries. Saving...", count);
    }
    tx.commit()?;

    println!("Creating index on sitelinks table...");
    conn.execute(
        "CREATE INDEX idx_sitelink_lookup ON sitelinks (lang, wiki, title);",
        [],
    )?;

    println!("Successfully created SQLite database at {}.", db_path);
    Ok(())
}

// Sorting
pub enum SortMode {
    Alphabetical,
    Numeric,
    XId,
}

pub fn external_merge_sort(
    input_path: &str,
    output_path: &str,
    mode: SortMode,
    ram_limit_mb: usize,
    num_threads: usize,
    delimiter: &str,
) -> io::Result<()> {
    if !Path::new(input_path).exists() {
        println!("Skipping (file not found): {}", input_path);
        return Ok(());
    }

    println!(
        "Starting system external sort for: {} (RAM: {} MB, Threads: {})",
        input_path, ram_limit_mb, num_threads
    );

    let mut cmd = Command::new("sort");

    cmd.env("LC_ALL", "C");

    let ram_arg = format!("{}M", ram_limit_mb);
    let threads_arg = format!("{}", num_threads);

    cmd.arg("-S")
        .arg(ram_arg)
        .arg("--parallel")
        .arg(threads_arg);

    cmd.arg("-t").arg(delimiter);

    match mode {
        SortMode::Alphabetical => {
            cmd.arg("-k").arg("1,1").arg("-f");
        }
        SortMode::Numeric => {
            cmd.arg("-k").arg("1,1g");
        }
        SortMode::XId => {
            cmd.arg("-k").arg("1.2,1n");
        }
    }

    cmd.arg("-o").arg(output_path);
    cmd.arg(input_path);

    let status = cmd.status()?;

    if status.success() {
        println!(
            "  -> Sorting successful! Deleting original file: {}",
            input_path
        );
        let _ = fs::remove_file(input_path);
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "System sort command failed",
        ))
    }
}

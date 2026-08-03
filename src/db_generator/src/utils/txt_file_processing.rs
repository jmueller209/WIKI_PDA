use redb::{Database, TableDefinition};
use std::fs;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::Command;

const SITELINKS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sitelinks");

pub fn build_sitelink_database(
    txt_path: &str,
    db_path: &str,
    delimiter: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file(&db_path);
    let db = Database::create(&db_path)?;
    let file = File::open(txt_path)?;
    let reader = io::BufReader::new(file);

    let mut count = 0;
    let batch_size = 2_000_000;

    let mut write_txn = db.begin_write()?;

    for line_result in reader.lines() {
        let line = line_result?;
        if let Some(last_delim_idx) = line.rfind(delimiter) {
            let key = &line[..last_delim_idx];
            let qid = &line[last_delim_idx + delimiter.len()..];
            {
                let mut table = write_txn.open_table(SITELINKS_TABLE)?;
                table.insert(key, qid)?;
            }
            count += 1;
            if count % batch_size == 0 {
                write_txn.commit()?;
                println!("  ... {} entries copied and committed", count);
                write_txn = db.begin_write()?;
            }
        }
    }

    write_txn.commit()?;
    println!("Imported a total of {} entries. Saving...", count);

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

    let safe_per_thread_ram = (ram_limit_mb / num_threads).max(64);
    let ram_arg = format!("{}M", safe_per_thread_ram);
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
        let in_abs = fs::canonicalize(input_path);
        let out_abs = fs::canonicalize(output_path);

        let is_same_file = match (in_abs, out_abs) {
            (Ok(in_path), Ok(out_path)) => in_path == out_path,
            _ => input_path == output_path,
        };
        if !is_same_file {
            println!(
                "  -> Sorting successful! Deleting original file: {}",
                input_path
            );
            let _ = fs::remove_file(input_path);
        } else {
            println!("  -> Sorting successful! (Sorted in-place, skipping deletion)");
        }

        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "System sort command failed",
        ))
    }
}

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use crate::utils::constants;
use crate::utils::settings::Settings;

const BLOCK_SIZE: usize = 512;

struct RemovableDrive {
    name: String,
    mount_point: String,
    total_space: u64,
}

pub fn cli(settings: &Settings) -> Result<(), String> {
    let default_db_dir = PathBuf::from_str(&settings.paths.bin_dir).unwrap();
    let default_db_path = default_db_dir.join(constants::DATA_BASE_BIN);

    println!("Scanning for connected removable media...");

    let safe_drives = get_removable_disks();
    if safe_drives.is_empty() {
        return Err("No removable drives found! Insert an SD card and try again.".to_string());
    }

    let max_menu_index = (safe_drives.len() - 1) as i64;

    loop {
        match get_index(max_menu_index) {
            MsgType::Idx(menu_choice) => {
                let selected_drive = &safe_drives[menu_choice as usize];
                let file_path = get_database_path(default_db_path.clone());
                let drive_label = get_drive_label();

                write_data(selected_drive, &file_path, &drive_label)?;
                break;
            }
            MsgType::Quit => return Ok(()),
            MsgType::Invalid(invalid_msg) => {
                println!("{}\n", invalid_msg);
                continue;
            }
        }
    }

    Ok(())
}

fn get_database_path(default_path: PathBuf) -> PathBuf {
    let msg = format!("Use default database path ({})?", default_path.display());

    match wait_for_agreement(&msg) {
        UserAgreement::Agree => {
            if default_path.is_file() {
                return default_path;
            } else {
                println!(
                    "Error: Default file does not exist at {}. You must specify a custom path.",
                    default_path.display()
                );
            }
        }
        UserAgreement::Disagree => {}
    }

    loop {
        print!("Enter custom path to database file: ");
        io::stdout().flush().unwrap();

        let mut custom_input = String::new();
        io::stdin().read_line(&mut custom_input).unwrap();
        let custom_path = PathBuf::from(custom_input.trim());

        if custom_path.is_file() {
            return custom_path;
        } else {
            println!("Error: File does not exist or is not a file. Please try again.\n");
        }
    }
}

fn get_drive_label() -> String {
    loop {
        print!("Enter a name for the drive (max 11 chars, leave empty for 'WIKIDRIVE'): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return "WIKIDRIVE".to_string();
        }

        let upper = trimmed.to_uppercase();
        if upper.len() <= 11 && upper.is_ascii() && !upper.contains(' ') {
            return upper;
        }

        println!("Invalid name. Must be 1-11 ASCII characters without spaces.\n");
    }
}

#[cfg(target_os = "linux")]
fn get_removable_disks() -> Vec<RemovableDrive> {
    let mut safe_drives = Vec::new();

    let Ok(paths) = std::fs::read_dir("/sys/block/") else {
        return safe_drives;
    };

    println!("===================================");
    for path in paths.flatten() {
        let name = path.file_name().into_string().unwrap_or_default();

        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("sr") {
            continue;
        }

        let removable_path = path.path().join("removable");
        if let Ok(removable_str) = std::fs::read_to_string(&removable_path) {
            if removable_str.trim() == "1" {
                let size_path = path.path().join("size");
                let size_sectors: u64 = std::fs::read_to_string(&size_path)
                    .unwrap_or_else(|_| "0".to_string())
                    .trim()
                    .parse()
                    .unwrap_or(0);

                let total_space = size_sectors * 512;

                if total_space == 0 {
                    continue;
                }

                let device_path = format!("/dev/{}", name);
                let mount_point =
                    get_mount_point(&device_path).unwrap_or_else(|| "Unmounted / Raw".to_string());

                println!("  [{}] Removable Hardware Drive Found!", safe_drives.len());
                println!("      Device: {}", device_path);
                println!("      State: {}", mount_point);
                println!("      Total Space: {} MB", total_space / 1_048_576);
                println!("-----------------------------------");

                safe_drives.push(RemovableDrive {
                    name: device_path,
                    mount_point,
                    total_space,
                });
            }
        }
    }
    safe_drives
}

#[cfg(target_os = "linux")]
fn get_mount_point(device_path: &str) -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0].starts_with(device_path) {
            return Some(parts[1].to_string());
        }
    }
    None
}

enum MsgType {
    Idx(i64),
    Quit,
    Invalid(String),
}

fn get_index(max_index: i64) -> MsgType {
    print!(
        "Select the drive index (0-{}) or type 'q' to quit: ",
        max_index
    );
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let choice_str = input.trim();
    if choice_str.eq_ignore_ascii_case("q") {
        return MsgType::Quit;
    }

    let Ok(choice) = choice_str.parse::<i64>() else {
        return MsgType::Invalid("Input was not a valid number.".to_string());
    };

    if choice >= 0 && choice <= max_index {
        MsgType::Idx(choice)
    } else {
        MsgType::Invalid(format!(
            "Index out of bounds. Must be between 0 and {}.",
            max_index
        ))
    }
}

enum UserAgreement {
    Agree,
    Disagree,
}

fn wait_for_agreement(msg: &str) -> UserAgreement {
    loop {
        print!("{} (y/n): ", msg);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return UserAgreement::Agree,
            "n" | "no" => return UserAgreement::Disagree,
            _ => println!("Invalid Answer. Please type 'y' or 'n'.\n"),
        }
    }
}

fn write_data(
    drive: &RemovableDrive,
    file_path: &PathBuf,
    drive_label: &str,
) -> Result<(), String> {
    let device_path = &drive.name;

    if std::env::var("USER").unwrap_or_default() != "root" {
        return Err("Raw flashing requires Administrator/root privileges.\nPlease run this command again using 'sudo'.".to_string());
    }

    let metadata =
        std::fs::metadata(file_path).map_err(|e| format!("Failed to read database file: {}", e))?;
    let db_size_bytes = metadata.len();

    let required_space = db_size_bytes + (32 * 1024 * 1024);

    if required_space > drive.total_space {
        return Err(format!(
            "SD Card is too small!\n  Database size: {} MB\n  Card capacity: {} MB\n  Required (with FAT32 overhead): ~{} MB",
            db_size_bytes / 1_048_576,
            drive.total_space / 1_048_576,
            required_space / 1_048_576
        ));
    }

    println!("\n================ DANGER ZONE ================");
    println!("Selected target: {}", device_path);
    println!("Mount point: {:?}", drive.mount_point);
    println!("Database size: {} MB", db_size_bytes / 1_048_576);

    let warning_msg = format!(
        "\nWARNING: You are about to perform a DESTRUCTIVE REPARTITION on {}.\n\
        This will PERMANENTLY OVERWRITE all data and filesystems on this drive.\n\
        Are you absolutely sure you want to proceed?",
        device_path
    );

    match wait_for_agreement(&warning_msg) {
        UserAgreement::Agree => {
            println!("Agreement confirmed. Preparing partitions...");

            let (fat32_partition, raw_partition) =
                setup_partitions(device_path, file_path, drive_label)?;

            println!("Partitions created successfully.");
            println!("Flashing database to {}...", raw_partition);

            flash_image_to_drive(file_path, &raw_partition).map_err(|e| e.to_string())?;

            write_fat32_metadata(&fat32_partition, db_size_bytes);

            println!("\nUpload complete! You can safely remove the SD card.");
            Ok(())
        }
        UserAgreement::Disagree => {
            println!("Operation aborted by user.");
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
fn setup_partitions(
    partition_path: &str,
    db_file_path: &Path,
    drive_label: &str,
) -> Result<(String, String), String> {
    let base_device = partition_path.trim_end_matches(|c: char| c.is_ascii_digit());
    let base_device = base_device.trim_end_matches('p');

    let metadata = std::fs::metadata(db_file_path).map_err(|e| e.to_string())?;
    let db_size_mb = (metadata.len() / (1024 * 1024)) + 5;

    println!(
        "Aggressively unmounting all partitions on {}...",
        base_device
    );
    for i in 1..=4 {
        let sd_part = format!("{}{}", base_device, i);
        let mmc_part = format!("{}p{}", base_device, i);
        Command::new("umount").arg(&sd_part).status().ok();
        Command::new("umount").arg(&mmc_part).status().ok();
        Command::new("umount").args(["-l", &sd_part]).status().ok();
        Command::new("umount").args(["-l", &mmc_part]).status().ok();
    }

    println!("Rewriting partition table via parted...");
    if let Err(e) = run_command("parted", &["-s", base_device, "mklabel", "msdos"]) {
        return Err(format!("Kernel lock error: {}", e));
    }

    let part1_end = format!("-{}MiB", db_size_mb);
    run_command(
        "parted",
        &[
            "-s",
            base_device,
            "--",
            "mkpart",
            "primary",
            "fat32",
            "1MiB",
            &part1_end,
        ],
    )?;
    run_command(
        "parted",
        &[
            "-s",
            base_device,
            "--",
            "mkpart",
            "primary",
            &part1_end,
            "100%",
        ],
    )?;

    let part1_path = format!("{}1", base_device);
    let part2_path = format!("{}2", base_device);

    println!(
        "Formatting Partition 1 ({}) as FAT32 with label '{}'...",
        part1_path, drive_label
    );

    run_command("mkfs.fat", &["-F", "32", "-n", drive_label, &part1_path])?;

    Ok((part1_path, part2_path))
}

#[cfg(target_os = "macos")]
fn setup_partitions(
    partition_path: &str,
    db_file_path: &Path,
    drive_label: &str,
) -> Result<String, String> {
    let base_device = partition_path.split('s').next().unwrap_or(partition_path);

    let metadata = std::fs::metadata(db_file_path).map_err(|e| e.to_string())?;
    let db_size_mb = (metadata.len() / (1024 * 1024)) + 5;
    let db_size_str = format!("{}M", db_size_mb);

    println!("Unmounting {}...", base_device);
    Command::new("diskutil")
        .args(["unmountDisk", base_device])
        .status()
        .ok();

    println!("Rewriting partition table via diskutil...");

    run_command(
        "diskutil",
        &[
            "partitionDisk",
            base_device,
            "2",
            "MBR",
            "FAT32",
            drive_label,
            "R",
            "MS-DOS",
            "RAW_DB",
            &db_size_str,
        ],
    )?;

    Ok(format!("{}s2", base_device))
}

#[cfg(target_os = "windows")]
fn setup_partitions(
    _partition_path: &str,
    _db_file_path: &Path,
    _drive_label: &str,
) -> Result<String, String> {
    Err("Automated partitioning on Windows is currently unsupported due to OS safety locks. Please use a Linux or macOS machine, or pre-partition the SD card manually.".to_string())
}

fn run_command(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

    if !status.success() {
        return Err(format!("Command '{} {:?}' failed.", cmd, args));
    }
    Ok(())
}

fn flash_image_to_drive(image_path: &Path, device_path: &str) -> io::Result<()> {
    println!("Opening source database file: {:?}", image_path);
    let mut source_file = File::open(image_path)?;
    let source_size = source_file.metadata()?.len();

    println!("Opening target partition: {}", device_path);

    let mut target_device = OpenOptions::new()
        .read(true)
        .write(true)
        .open(device_path)?;

    let mut buffer = vec![0u8; 4 * 1024 * 1024];
    let mut total_written: u64 = 0;

    loop {
        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let remainder = bytes_read % BLOCK_SIZE;
        let write_len = if remainder != 0 {
            let padded_len = bytes_read + (BLOCK_SIZE - remainder);
            for i in bytes_read..padded_len {
                buffer[i] = 0;
            }
            padded_len
        } else {
            bytes_read
        };

        target_device.write_all(&buffer[..write_len])?;
        total_written += bytes_read as u64;

        let progress = (total_written as f64 / source_size as f64) * 100.0;
        print!(
            "\rProgress: {:.2}% ({}/{} bytes)",
            progress, total_written, source_size
        );
        io::stdout().flush()?;
    }

    println!("\nFlushing OS buffers to SD card hardware...");
    target_device.sync_all()?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn write_fat32_metadata(part1_path: &str, db_size_bytes: u64) {
    println!("Generating README on the FAT32 partition...");
    let mount_dir = "/tmp/wiki_drive_mount";

    let _ = std::fs::create_dir_all(mount_dir);

    if run_command("mount", &[part1_path, mount_dir]).is_err() {
        println!("  Warning: Could not mount FAT32 partition to write README.");
        return;
    }

    let readme_path = format!("{}/README.txt", mount_dir);

    let readme_content = format!(
        "=======================================\n\
         WIKI DATABASE DRIVE\n\
         =======================================\n\n\
         STATUS: SUCCESS\n\
         The Wiki Database has been successfully flashed to this SD card.\n\n\
         DATABASE SIZE: {} MB\n\n\
         --- WHERE IS MY DATA? ---\n\
         You might notice that this drive looks mostly empty. \n\
         This is intentional!\n\n\
         To optimize read speeds for the microcontroller, the actual database \n\
         was written to a hidden, RAW partition (Partition 2) at the very \n\
         end of this SD card. It bypasses the filesystem entirely.\n\n\
         This visible FAT32 partition is left available for you to store \n\
         configuration files, logs, or other general data.",
        db_size_bytes / 1_048_576
    );

    if let Err(e) = std::fs::write(&readme_path, readme_content) {
        println!("  Warning: Failed to write README.txt: {}", e);
    } else {
        println!("  README.txt successfully generated.");
    }

    let _ = run_command("umount", &[mount_dir]);
    let _ = std::fs::remove_dir(mount_dir);
}

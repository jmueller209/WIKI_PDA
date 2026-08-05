use crate::utils::settings::Settings;
use sysinfo::Disks;

pub fn get_disks(settings: &Settings) -> Result<(), String> {
    // Load all connected disks
    let disks = Disks::new_with_refreshed_list();

    println!("Scanning for connected storage media...");

    for disk in disks.list() {
        if disk.is_removable() {
            println!("Found Removable Drive!");
            println!("  Name: {:?}", disk.name());
            println!("  Mount Point: {:?}", disk.mount_point());
            println!("  Total Space: {} bytes", disk.total_space());
            println!("-----------------------------------");
        } else {
            println!("Found Non Removable Drive!");
            println!("  Name: {:?}", disk.name());
            println!("  Mount Point: {:?}", disk.mount_point());
            println!("  Total Space: {} bytes", disk.total_space());
            println!("-----------------------------------");
        }
    }
    Ok(())
}

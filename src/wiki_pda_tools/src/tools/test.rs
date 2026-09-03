use crate::utils::settings::Settings;
use crate::utils::tagging;
use std::collections::HashMap;
use std::time::Instant;

pub fn test(settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Starting Tag Dictionary Test ===\n");

    // --- 1. Fetch or Load the Dictionary ---
    println!("Step 1: Initializing Dictionary...");
    let start_time = Instant::now();
    let (tag_dictionary, metrics) = tagging::get_or_create_tag_dictionary(settings)?;
    let time_taken = start_time.elapsed();

    println!("\n--- Dictionary Metrics ---");
    println!("Status: {}", metrics.cache_status);
    println!("Loaded from Cache: {}", metrics.loaded_from_cache);
    println!("Total Mappings: {}", metrics.total_mappings);
    println!("Rate Limit Hits: {}", metrics.rate_limit_hits);
    if !metrics.failed_tags.is_empty() {
        println!("Failed Tags: {:?}", metrics.failed_tags);
    }
    println!("Time taken: {:?}", time_taken);
    println!("--------------------------\n");

    // --- 2. Validate Data Structure ---
    println!("Step 2: Validating Data Structure...");
    let mut grouped_by_parent: HashMap<&String, Vec<&String>> = HashMap::new();

    // Group subclasses by their parents to verify the overlaps worked
    for (subclass, parents) in &tag_dictionary {
        for parent in parents {
            grouped_by_parent
                .entry(parent)
                .or_insert_with(Vec::new)
                .push(subclass);
        }
    }

    // We'll just print the Omni tags to keep the terminal output clean
    let core_tags = &settings.database_content.omni_search_index_tags;
    println!("\n--- Core Tag Association Summary ---");
    for parent in core_tags {
        if let Some(subclasses) = grouped_by_parent.get(parent) {
            println!(
                "Parent {}: Found {} associated subclasses",
                parent,
                subclasses.len()
            );

            let sample: Vec<String> = subclasses.iter().take(5).map(|s| s.to_string()).collect();
            println!("  -> Previews: {:?}", sample);
        } else {
            println!("Parent {}: No subclasses found (or skipped).", parent);
        }
    }
    println!("------------------------------------\n");

    // --- 3. Test Cache Speed ---
    println!("Step 3: Testing Cache Speed...");
    let cache_start_time = Instant::now();
    let (_, cache_metrics) = tagging::get_or_create_tag_dictionary(settings)?;
    let cache_time_taken = cache_start_time.elapsed();

    println!("Second run time: {:?}", cache_time_taken);

    if cache_metrics.loaded_from_cache {
        println!("✅ Cache test passed! The dictionary loaded instantly.");
    } else {
        println!("❌ Cache test failed! It tried to rebuild the dictionary.");
    }

    println!("\n=== Tag Dictionary Test Complete ===");
    Ok(())
}

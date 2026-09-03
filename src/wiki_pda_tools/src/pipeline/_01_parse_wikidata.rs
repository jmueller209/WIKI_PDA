use crossbeam_channel::bounded;
use flate2::read::MultiGzDecoder;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write as IoWrite};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::utils::checkpoints;
use crate::utils::constants;
use crate::utils::encoding;
use crate::utils::logs;
use crate::utils::settings::Settings;
use crate::utils::txt_file_processing::{self, SortMode};

#[derive(Default, Debug)]
pub struct ParserMetrics {
    pub num_lines_read: u64,
    pub num_lines_skipped: u64,

    pub qids_found_total: u64,
    pub qids_used_total: u64,

    // OMNI SEARCH
    pub qids_used_in_omni_search: u64,
    pub omni_search_entries_created: u64,
    pub qids_used_in_omni_search_with_article_total: u64,
    pub qids_used_in_omni_search_no_article_total: u64,
    pub qids_used_in_omni_search_by_lang: HashMap<String, u64>,
    pub tag_usage_in_omni: HashMap<String, u64>,

    // GLOBE COORDINATES
    pub qids_used_in_coordinate_search: u64,
    pub qids_used_in_coordinate_search_with_article_total: u64,
    pub qids_used_in_coordinate_search_without_article: u64,
    pub qids_used_in_coordinate_search_by_lang: HashMap<String, u64>,
    pub tag_usage_in_globe: HashMap<String, u64>,

    // TEMPORAL
    pub qids_used_in_temporal_search: u64,
    pub qids_used_in_temporal_search_with_article_total: u64,
    pub qids_used_in_temporal_search_without_article: u64,
    pub qids_used_in_temporal_search_by_lang: HashMap<String, u64>,
    pub tag_usage_in_temporal: HashMap<String, u64>,

    // ASTRONOMICAL
    pub qids_used_in_astronomical_search: u64,
    pub qids_used_in_astronomical_search_with_article_total: u64,
    pub qids_used_in_astronomical_search_without_article: u64,
    pub qids_used_in_astronomical_search_by_lang: HashMap<String, u64>,
    pub tag_usage_in_astro: HashMap<String, u64>,

    pub metadata_entries_written: u64,
    pub empty_metadata_entries_written: u64,

    pub pids_found: u64,
    pub pids_used: u64,
    pub property_usage_count: HashMap<String, u64>,
}

impl ParserMetrics {
    pub fn merge(&mut self, other: Self) {
        self.num_lines_read += other.num_lines_read;
        self.num_lines_skipped += other.num_lines_skipped;
        self.qids_found_total += other.qids_found_total;
        self.qids_used_total += other.qids_used_total;

        // OMNI
        self.qids_used_in_omni_search += other.qids_used_in_omni_search;
        self.omni_search_entries_created += other.omni_search_entries_created;
        self.qids_used_in_omni_search_with_article_total +=
            other.qids_used_in_omni_search_with_article_total;
        self.qids_used_in_omni_search_no_article_total +=
            other.qids_used_in_omni_search_no_article_total;
        for (k, v) in other.qids_used_in_omni_search_by_lang {
            *self.qids_used_in_omni_search_by_lang.entry(k).or_insert(0) += v;
        }
        for (k, v) in other.tag_usage_in_omni {
            *self.tag_usage_in_omni.entry(k).or_insert(0) += v;
        }

        // GLOBE
        self.qids_used_in_coordinate_search += other.qids_used_in_coordinate_search;
        self.qids_used_in_coordinate_search_with_article_total +=
            other.qids_used_in_coordinate_search_with_article_total;
        self.qids_used_in_coordinate_search_without_article +=
            other.qids_used_in_coordinate_search_without_article;
        for (k, v) in other.qids_used_in_coordinate_search_by_lang {
            *self
                .qids_used_in_coordinate_search_by_lang
                .entry(k)
                .or_insert(0) += v;
        }
        for (k, v) in other.tag_usage_in_globe {
            *self.tag_usage_in_globe.entry(k).or_insert(0) += v;
        }

        // TEMPORAL
        self.qids_used_in_temporal_search += other.qids_used_in_temporal_search;
        self.qids_used_in_temporal_search_with_article_total +=
            other.qids_used_in_temporal_search_with_article_total;
        self.qids_used_in_temporal_search_without_article +=
            other.qids_used_in_temporal_search_without_article;
        for (k, v) in other.qids_used_in_temporal_search_by_lang {
            *self
                .qids_used_in_temporal_search_by_lang
                .entry(k)
                .or_insert(0) += v;
        }
        for (k, v) in other.tag_usage_in_temporal {
            *self.tag_usage_in_temporal.entry(k).or_insert(0) += v;
        }

        // ASTRO
        self.qids_used_in_astronomical_search += other.qids_used_in_astronomical_search;
        self.qids_used_in_astronomical_search_with_article_total +=
            other.qids_used_in_astronomical_search_with_article_total;
        self.qids_used_in_astronomical_search_without_article +=
            other.qids_used_in_astronomical_search_without_article;
        for (k, v) in other.qids_used_in_astronomical_search_by_lang {
            *self
                .qids_used_in_astronomical_search_by_lang
                .entry(k)
                .or_insert(0) += v;
        }
        for (k, v) in other.tag_usage_in_astro {
            *self.tag_usage_in_astro.entry(k).or_insert(0) += v;
        }

        self.metadata_entries_written += other.metadata_entries_written;
        self.empty_metadata_entries_written += other.empty_metadata_entries_written;
        self.pids_found += other.pids_found;
        self.pids_used += other.pids_used;

        for (k, v) in other.property_usage_count {
            *self.property_usage_count.entry(k).or_insert(0) += v;
        }
    }

    pub fn make_summary(&self) -> String {
        let mut summary = String::new();

        // Helper function to print top tags for any index
        let print_top_tags = |summary: &mut String, tags: &HashMap<String, u64>, title: &str| {
            if !tags.is_empty() {
                writeln!(summary, "  -> {}:", title).unwrap();
                let mut tag_vec: Vec<(&String, &u64)> = tags.iter().collect();
                tag_vec.sort_by(|a, b| b.1.cmp(a.1)); // Sort descending
                for (tag, count) in tag_vec.iter().take(10) {
                    writeln!(summary, "      - {}: {} matches", tag, count).unwrap();
                }
            }
        };

        // Helper function to print languages
        let print_langs = |summary: &mut String, langs: &HashMap<String, u64>| {
            if !langs.is_empty() {
                writeln!(summary, "     (Breakdown by language)").unwrap();
                let mut lang_vec: Vec<(&String, &u64)> = langs.iter().collect();
                lang_vec.sort_by(|a, b| b.1.cmp(a.1));
                for (lang, count) in lang_vec {
                    writeln!(summary, "      - {}: {} matches", lang, count).unwrap();
                }
            }
        };

        writeln!(
            &mut summary,
            "\n================ PARSING SUMMARY ================"
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Total Lines Read:                         {}",
            self.num_lines_read
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Total Lines Skipped:                      {}",
            self.num_lines_skipped
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Unique QIDs Found:                        {}",
            self.qids_found_total
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Unique QIDs Used (In Any Index):          {}",
            self.qids_used_total
        )
        .unwrap();
        writeln!(
            &mut summary,
            "P-IDs Found:                              {}",
            self.pids_found
        )
        .unwrap();
        writeln!(
            &mut summary,
            "P-IDs Added (Psearch):                    {}",
            self.pids_used
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Metadata Entries Written (Total):         {}",
            self.metadata_entries_written
        )
        .unwrap();
        writeln!(
            &mut summary,
            "Empty Metadata Entries Written:           {}",
            self.empty_metadata_entries_written
        )
        .unwrap();

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(&mut summary, "INDEX: OMNI SEARCH").unwrap();
        writeln!(
            &mut summary,
            "  Total Unique QIDs:                      {}",
            self.qids_used_in_omni_search
        )
        .unwrap();
        writeln!(
            &mut summary,
            "  Total Entries Created (Lines):          {}",
            self.omni_search_entries_created
        )
        .unwrap();
        writeln!(
            &mut summary,
            "  -> Unique QIDs with Wikipedia Article:  {}",
            self.qids_used_in_omni_search_with_article_total
        )
        .unwrap();
        print_langs(&mut summary, &self.qids_used_in_omni_search_by_lang);
        writeln!(
            &mut summary,
            "  -> Unique QIDs via Concept (No Article):{}",
            self.qids_used_in_omni_search_no_article_total
        )
        .unwrap();
        print_top_tags(
            &mut summary,
            &self.tag_usage_in_omni,
            "Top 10 Tags Used in Omni",
        );

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(&mut summary, "INDEX: GLOBE COORDINATES").unwrap();
        writeln!(
            &mut summary,
            "  Total Unique QIDs:                      {}",
            self.qids_used_in_coordinate_search
        )
        .unwrap();
        writeln!(
            &mut summary,
            "  -> Unique QIDs with Wikipedia Article:  {}",
            self.qids_used_in_coordinate_search_with_article_total
        )
        .unwrap();
        print_langs(&mut summary, &self.qids_used_in_coordinate_search_by_lang);
        writeln!(
            &mut summary,
            "  -> Unique QIDs Without Article:         {}",
            self.qids_used_in_coordinate_search_without_article
        )
        .unwrap();
        print_top_tags(
            &mut summary,
            &self.tag_usage_in_globe,
            "Top 10 Tags Used in Globe",
        );

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(&mut summary, "INDEX: TEMPORAL").unwrap();
        writeln!(
            &mut summary,
            "  Total Unique QIDs:                      {}",
            self.qids_used_in_temporal_search
        )
        .unwrap();
        writeln!(
            &mut summary,
            "  -> Unique QIDs with Wikipedia Article:  {}",
            self.qids_used_in_temporal_search_with_article_total
        )
        .unwrap();
        print_langs(&mut summary, &self.qids_used_in_temporal_search_by_lang);
        writeln!(
            &mut summary,
            "  -> Unique QIDs Without Article:         {}",
            self.qids_used_in_temporal_search_without_article
        )
        .unwrap();
        print_top_tags(
            &mut summary,
            &self.tag_usage_in_temporal,
            "Top 10 Tags Used in Temporal",
        );

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(&mut summary, "INDEX: ASTRONOMICAL").unwrap();
        writeln!(
            &mut summary,
            "  Total Unique QIDs:                      {}",
            self.qids_used_in_astronomical_search
        )
        .unwrap();
        writeln!(
            &mut summary,
            "  -> Unique QIDs with Wikipedia Article:  {}",
            self.qids_used_in_astronomical_search_with_article_total
        )
        .unwrap();
        print_langs(&mut summary, &self.qids_used_in_astronomical_search_by_lang);
        writeln!(
            &mut summary,
            "  -> Unique QIDs Without Article:         {}",
            self.qids_used_in_astronomical_search_without_article
        )
        .unwrap();
        print_top_tags(
            &mut summary,
            &self.tag_usage_in_astro,
            "Top 10 Tags Used in Astronomical",
        );

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(
            &mut summary,
            "TOP 25 MOST USED PROPERTIES IN EXPORTED METADATA:"
        )
        .unwrap();
        let mut prop_vec: Vec<(&String, &u64)> = self.property_usage_count.iter().collect();
        prop_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (rank, (prop, count)) in prop_vec.iter().take(25).enumerate() {
            writeln!(&mut summary, "{:>2}. {}: {} times", rank + 1, prop, count).unwrap();
        }

        writeln!(
            &mut summary,
            "--------------------------------------------------------"
        )
        .unwrap();
        writeln!(
            &mut summary,
            "SANITY CHECKS (Mathematical Integrity Verification):"
        )
        .unwrap();

        let meta_ok = self.metadata_entries_written == self.qids_used_total;
        writeln!(
            &mut summary,
            " - Metadata Consistency: [{}] ({} metadata == {} used QIDs)",
            if meta_ok { "OK" } else { "FAIL" },
            self.metadata_entries_written,
            self.qids_used_total
        )
        .unwrap();

        let omni_ok = (self.qids_used_in_omni_search_with_article_total
            + self.qids_used_in_omni_search_no_article_total)
            == self.qids_used_in_omni_search;
        writeln!(
            &mut summary,
            " - Omni Math Match:      [{}] ({} article + {} no_article == {})",
            if omni_ok { "OK" } else { "FAIL" },
            self.qids_used_in_omni_search_with_article_total,
            self.qids_used_in_omni_search_no_article_total,
            self.qids_used_in_omni_search
        )
        .unwrap();

        let globe_ok = (self.qids_used_in_coordinate_search_with_article_total
            + self.qids_used_in_coordinate_search_without_article)
            == self.qids_used_in_coordinate_search;
        writeln!(
            &mut summary,
            " - Globe Math Match:     [{}] ({} article + {} no_article == {})",
            if globe_ok { "OK" } else { "FAIL" },
            self.qids_used_in_coordinate_search_with_article_total,
            self.qids_used_in_coordinate_search_without_article,
            self.qids_used_in_coordinate_search
        )
        .unwrap();

        let temp_ok = (self.qids_used_in_temporal_search_with_article_total
            + self.qids_used_in_temporal_search_without_article)
            == self.qids_used_in_temporal_search;
        writeln!(
            &mut summary,
            " - Temporal Math Match:  [{}] ({} article + {} no_article == {})",
            if temp_ok { "OK" } else { "FAIL" },
            self.qids_used_in_temporal_search_with_article_total,
            self.qids_used_in_temporal_search_without_article,
            self.qids_used_in_temporal_search
        )
        .unwrap();

        let astro_ok = (self.qids_used_in_astronomical_search_with_article_total
            + self.qids_used_in_astronomical_search_without_article)
            == self.qids_used_in_astronomical_search;
        writeln!(
            &mut summary,
            " - Astro Math Match:     [{}] ({} article + {} no_article == {})",
            if astro_ok { "OK" } else { "FAIL" },
            self.qids_used_in_astronomical_search_with_article_total,
            self.qids_used_in_astronomical_search_without_article,
            self.qids_used_in_astronomical_search
        )
        .unwrap();

        let lines_sum = self.qids_used_total + self.pids_used + self.num_lines_skipped;
        let lines_ok = lines_sum == self.num_lines_read;
        writeln!(
            &mut summary,
            " - Total Lines Match:    [{}] ({} used QIDs + {} used PIDs + {} skipped == {})",
            if lines_ok { "OK" } else { "FAIL" },
            self.qids_used_total,
            self.pids_used,
            self.num_lines_skipped,
            self.num_lines_read
        )
        .unwrap();

        writeln!(
            &mut summary,
            "========================================================"
        )
        .unwrap();

        summary
    }
}
struct PreparedBatch {
    omni_search_lines: String,
    metadata_lines: String,
    properties_lines: String,
    sitelinks_mapping_lines: String,
    coordinate_lines: String,
    astronomical_lines: String,
    temporal_lines: String,
}

impl PreparedBatch {
    fn empty() -> Self {
        Self {
            omni_search_lines: String::new(),
            metadata_lines: String::new(),
            properties_lines: String::new(),
            sitelinks_mapping_lines: String::new(),
            coordinate_lines: String::new(),
            astronomical_lines: String::new(),
            temporal_lines: String::new(),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.omni_search_lines.push_str(&other.omni_search_lines);
        self.metadata_lines.push_str(&other.metadata_lines);
        self.properties_lines.push_str(&other.properties_lines);
        self.sitelinks_mapping_lines
            .push_str(&other.sitelinks_mapping_lines);
        self.coordinate_lines.push_str(&other.coordinate_lines);
        self.astronomical_lines.push_str(&other.astronomical_lines);
        self.temporal_lines.push_str(&other.temporal_lines);
        self
    }
}

pub fn parse_wikidata(settings: &Settings, max_test_lines: Option<usize>) -> Result<(), String> {
    match checkpoints::checkpoint_exists(&settings, 1) {
        checkpoints::CheckpointState::ExistsEmpty => {
            println!("Checkpoint found: Wikidata parser has already finished");
            return Ok(());
        }
        checkpoints::CheckpointState::ExistsWithData(data) => {
            return Err(format!(
                "Download checkpoint should not contain any data, but contains: \n {}",
                data
            ));
        }
        checkpoints::CheckpointState::ExistsInBadState(i) => {
            let _ = checkpoints::clear_checkpoints(&settings, i);
            return Err("Checkpoint was found in bad state. Cleaned up checkpoints.".to_string());
        }
        checkpoints::CheckpointState::DoesNotExist => (),
    }

    let num_threads = settings.performance.thread_count;
    let read_buffer_bytes = settings.performance.read_buffer_size_kb * 1024;
    let write_buffer_bytes = settings.performance.write_buffer_size_kb * 1024;

    let ram_limit_mb = settings.performance.ram_limit_mb;
    let text_delimiter = &settings.other.text_delimiter;

    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    let include_concepts_with_given_property_in_omni_search_index: HashSet<String> = settings
        .database_content
        .include_concepts_with_given_property_in_omni_search_index
        .clone()
        .into_iter()
        .collect();
    let omni_search_index_tags: HashSet<String> = settings
        .database_content
        .omni_search_index_tags
        .clone()
        .into_iter()
        .collect();

    let omni_search_index_case_sensitive =
        settings.database_content.omni_search_index_case_sensitive;

    let create_globe_coordinate_search_index = settings
        .database_content
        .create_globe_coordinate_search_index;
    let include_all_matches_in_globe_coordinate_search_index = settings
        .database_content
        .include_all_matches_in_globe_coordinate_search_index;
    let globe_coordinate_search_index_tags: HashSet<String> = settings
        .database_content
        .globe_coordinate_search_index_tags
        .iter()
        .cloned()
        .collect();

    let create_temporal_search_index = settings.database_content.create_temporal_search_index;
    let include_all_matches_in_temporal_search_index = settings
        .database_content
        .include_all_matches_in_temporal_search_index;
    let temporal_search_index_tags: HashSet<String> = settings
        .database_content
        .temporal_search_index_tags
        .iter()
        .cloned()
        .collect();

    let create_astronomical_search_index =
        settings.database_content.create_astronomical_search_index;
    let include_all_matches_in_astronomical_search_index = settings
        .database_content
        .include_all_matches_in_astronomical_search_index;
    let astronomical_search_index_tags: HashSet<String> = settings
        .database_content
        .astronomical_search_index_tags
        .iter()
        .cloned()
        .collect();
    let astronomical_objects_to_include: HashSet<String> = settings
        .database_content
        .astronomical_objects_to_include
        .iter()
        .cloned()
        .collect();
    let max_apparent_magnitude = settings.database_content.max_apparent_magnitude;

    let property_datatypes_to_include_in_metadata: HashSet<String> = settings
        .database_content
        .property_datatypes_to_include_in_metadata
        .clone()
        .into_iter()
        .collect();

    let languages_to_include: HashSet<String> = settings
        .database_content
        .language_to_include
        .iter()
        .map(|lang| lang.as_str().to_string())
        .collect();

    let input_file = File::open(&settings.paths.wikidata_dump_path)
        .expect("Failed to open the Wikidata dump file.");
    let disk_buffer = BufReader::with_capacity(read_buffer_bytes, input_file);
    let decoder = MultiGzDecoder::new(disk_buffer);
    let reader = BufReader::with_capacity(read_buffer_bytes, decoder);
    let global_metrics = Arc::new(Mutex::new(ParserMetrics::default()));

    let tmp_dir = PathBuf::from(&settings.paths.tmp_dir);
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    fn unsorted_filename(filename: &str) -> String {
        filename.replace(".txt", "_unsorted.txt")
    }

    let omni_search_unsorted_txt_path = tmp_dir.join(unsorted_filename(constants::OMNI_SEARCH_TXT));
    let omni_search_txt_path = tmp_dir.join(constants::OMNI_SEARCH_TXT);
    let mut omni_search_file = BufWriter::with_capacity(
        write_buffer_bytes,
        File::create(&omni_search_unsorted_txt_path).unwrap(),
    );

    let properties_search_unsorted_txt_path =
        tmp_dir.join(unsorted_filename(constants::PROPERTIES_SEARCH_TXT));
    let properties_search_txt_path = tmp_dir.join(constants::PROPERTIES_SEARCH_TXT);
    let mut properties_search_file = BufWriter::with_capacity(
        write_buffer_bytes,
        File::create(&properties_search_unsorted_txt_path).unwrap(),
    );

    let sitelinks_qid_mapping_raw_unsorted_txt_path =
        tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB.replace(".db", "_raw_unsorted.txt"));
    let sitelinks_qid_mapping_raw_txt_path =
        tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB.replace(".db", "_raw.txt"));
    let sitelinks_qid_mapping_db_path = tmp_dir.join(constants::SITELINKS_QID_MAPPING_DB);
    let mut sitelinks_qid_mapping_file = BufWriter::with_capacity(
        write_buffer_bytes,
        File::create(&sitelinks_qid_mapping_raw_unsorted_txt_path).unwrap(),
    );

    let meta_data_unsorted_txt_path = tmp_dir.join(unsorted_filename(constants::META_DATA_TXT));
    let meta_data_txt_path = tmp_dir.join(constants::META_DATA_TXT);
    let mut meta_data_file = BufWriter::with_capacity(
        write_buffer_bytes,
        File::create(&meta_data_unsorted_txt_path).unwrap(),
    );

    let globe_coordinate_search_unsorted_txt_path =
        tmp_dir.join(unsorted_filename(constants::GLOBE_COORDINATE_SEARCH_TXT));
    let globe_coordinate_search_txt_path = tmp_dir.join(constants::GLOBE_COORDINATE_SEARCH_TXT);
    let mut globe_coordinate_search_file = if create_globe_coordinate_search_index {
        Some(BufWriter::with_capacity(
            write_buffer_bytes,
            File::create(&globe_coordinate_search_unsorted_txt_path).unwrap(),
        ))
    } else {
        None
    };

    let astronomical_search_unsorted_txt_path =
        tmp_dir.join(unsorted_filename(constants::ASTRONOMICAL_SEARCH_TXT));
    let astronomical_search_txt_path = tmp_dir.join(constants::ASTRONOMICAL_SEARCH_TXT);
    let mut astronomical_search_file = if create_astronomical_search_index {
        Some(BufWriter::with_capacity(
            write_buffer_bytes,
            File::create(&astronomical_search_unsorted_txt_path).unwrap(),
        ))
    } else {
        None
    };

    let temporal_search_unsorted_txt_path =
        tmp_dir.join(unsorted_filename(constants::TEMPORAL_SEARCH_TXT));
    let temporal_search_txt_path = tmp_dir.join(constants::TEMPORAL_SEARCH_TXT);
    let mut temporal_search_file = if create_temporal_search_index {
        Some(BufWriter::with_capacity(
            write_buffer_bytes,
            File::create(&temporal_search_unsorted_txt_path).unwrap(),
        ))
    } else {
        None
    };

    println!("Initializing unified tag dictionary...");
    let (tag_dict_raw, tag_metrics) = crate::utils::tagging::get_or_create_tag_dictionary(settings)
        .expect("Failed to initialize tag dictionary");
    println!(
        "Tag dictionary ready: {} metrics. {}",
        tag_dict_raw.len(),
        tag_metrics.cache_status
    );
    let shared_tag_dict = Arc::new(tag_dict_raw);

    let batch_size = 10_000;
    let (raw_tx, raw_rx) = bounded::<Vec<String>>(10);
    let (parsed_tx, parsed_rx) = bounded::<PreparedBatch>(10);
    println!(
        "Starting multi-threaded pipeline using {} threads...",
        num_threads
    );

    thread::scope(|s| {
        s.spawn(move || {
            let mut current_batch = Vec::with_capacity(batch_size);
            let mut line_count = 0;

            println!("Reader Thread: Starting to decompress gzip... (this may take a few seconds)");

            for (index, line_result) in reader.lines().enumerate() {
                let line = match line_result {
                    Ok(l) => l,
                    Err(e) => {
                        println!("FATAL READ ERROR at line {}: {}", index, e);
                        break;
                    }
                };

                if index == 0 {
                    println!("Reader Thread: Successfully unzipped the first line! It looks like this: {:.30}...", line);
                }

                if line.len() < 5 {
                    continue;
                }

                current_batch.push(line);
                line_count += 1;

                if line_count % 2000 == 0 {
                    println!("Reader Thread: Extracted {} lines so far...", line_count);
                }

                if current_batch.len() >= batch_size {
                    raw_tx.send(current_batch).unwrap();
                    current_batch = Vec::with_capacity(batch_size);
                }

                if let Some(max_lines) = max_test_lines {
                    if line_count >= max_lines {
                        println!("Test limit reached ({} lines). Stopping reader.", max_lines);
                        break;
                    }
                }
            }
            if !current_batch.is_empty() {
                raw_tx.send(current_batch).unwrap();
            }
        });
        s.spawn(move || {
            for batch in parsed_rx {
                if !batch.omni_search_lines.is_empty() {
                    omni_search_file
                        .write_all(batch.omni_search_lines.as_bytes())
                        .unwrap();
                }
                if !batch.metadata_lines.is_empty() {
                    meta_data_file
                        .write_all(batch.metadata_lines.as_bytes())
                        .unwrap();
                }
                if !batch.properties_lines.is_empty() {
                    properties_search_file
                        .write_all(batch.properties_lines.as_bytes())
                        .unwrap();
                }
                if !batch.sitelinks_mapping_lines.is_empty() {
                    sitelinks_qid_mapping_file
                        .write_all(batch.sitelinks_mapping_lines.as_bytes())
                        .unwrap();
                }

                if !batch.coordinate_lines.is_empty() {
                    if let Some(ref mut writer) = globe_coordinate_search_file {
                        writer.write_all(batch.coordinate_lines.as_bytes()).unwrap();
                    }
                }
                if !batch.astronomical_lines.is_empty() {
                    if let Some(ref mut writer) = astronomical_search_file {
                        writer
                            .write_all(batch.astronomical_lines.as_bytes())
                            .unwrap();
                    }
                }
                if !batch.temporal_lines.is_empty() {
                    if let Some(ref mut writer) = temporal_search_file {
                        writer.write_all(batch.temporal_lines.as_bytes()).unwrap();
                    }
                }
            }
            println!("Writer Thread: All data written to disk safely.");
        });

        let global_metrics_clone = Arc::clone(&global_metrics);
        let tag_dict_clone = Arc::clone(&shared_tag_dict);

        let earth_ctx = encoding::safe_spatial_create_earth_ctx();
        let celestial_ctx = encoding::safe_spatial_create_celestial_ctx();
        for batch in raw_rx {
            let batch_len = batch.len() as u64;
            let (processed_batch, batch_metrics) = batch
                .into_par_iter()
                .filter_map(|line| {
                    let mut local_metrics = ParserMetrics::default();

                    let trimmed = line.trim();
                    let clean_line = if trimmed.ends_with(',') {
                        &trimmed[..trimmed.len() - 1]
                    } else {
                        trimmed
                    };

                    let parsed: serde_json::Value = match serde_json::from_str(clean_line) {
                        Ok(val) => val,
                        Err(e) => {
                            local_metrics.num_lines_skipped += 1;
                            println!("JSON parse error: {}. Skipping line.", e);
                            return Some((PreparedBatch::empty(), local_metrics));
                        }
                    };

                    let Some(entity_id) = parsed["id"].as_str() else {
                        local_metrics.num_lines_skipped += 1;
                        return Some((PreparedBatch::empty(), local_metrics));
                    };

                    let is_q_item = entity_id.starts_with('Q');
                    let is_p_property = entity_id.starts_with('P');

                    if is_q_item {
                        local_metrics.qids_found_total += 1;
                        let mut has_relevant_sitelink = false;
                        let mut valid_sitelinks = Vec::new();

                        if let Some(sitelinks) = parsed["sitelinks"].as_object() {
                            for (site_key, site_data) in sitelinks {
                                if site_key.ends_with("wiki") {
                                    let lang_code_len = site_key.len() - 4;
                                    let lang_code = &site_key[..lang_code_len];

                                    if languages_to_include.contains(lang_code) {
                                        has_relevant_sitelink = true;

                                        if let Some(title) = site_data["title"].as_str() {
                                            valid_sitelinks.push((
                                                lang_code.to_string(),
                                                title.to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        let unique_langs: HashSet<String> = valid_sitelinks.iter().map(|(l, _)| l.clone()).collect();
                        let mut has_included_concept = false;
                        let mut p31_qids = Vec::new();
                        let mut matched_omni_concepts = Vec::new();
                        let mut matched_astro_concepts = Vec::new();

                        if let Some(claims) = parsed["claims"].as_object() {
                            if let Some(p31_array) = claims.get("P31").and_then(|v| v.as_array()) {
                                for claim in p31_array {
                                    if let Some(id) = claim
                                        .pointer("/mainsnak/datavalue/value/id")
                                        .and_then(|v| v.as_str())
                                    {
                                        let id_str = id.to_string();
                                        p31_qids.push(id_str.clone());

                                        if include_concepts_with_given_property_in_omni_search_index.contains(&id_str) {
                                            has_included_concept = true;
                                            matched_omni_concepts.push(id_str.clone());
                                        }
                                        if create_astronomical_search_index && astronomical_objects_to_include.contains(&id_str) {
                                            matched_astro_concepts.push(id_str.clone());
                                        }
                                    }
                                }
                            }
                        }

                        let mut entity_data = PreparedBatch::empty();
                        let mut grouped_claims: HashMap<String, Vec<String>> = HashMap::new();
                        let mut export_claims: HashMap<String, Vec<String>> = HashMap::new();

                        if let Some(claims) = parsed["claims"].as_object() {
                            for (prop_id, claim_array) in claims {
                                if let Some(arr) = claim_array.as_array() {
                                    for claim in arr {
                                        if claim["rank"].as_str() == Some("deprecated") {
                                            continue;
                                        }
                                        let mainsnak = &claim["mainsnak"];
                                        if mainsnak["snaktype"].as_str() != Some("value") {
                                            continue;
                                        }

                                        if let Some(datatype) = mainsnak["datatype"].as_str() {
                                            let datavalue_obj = &mainsnak["datavalue"]["value"];

                                            let extracted_value: Option<String> = match datatype {
                                                "string" | "external-id" | "commonsMedia"
                                                | "url" | "math" | "musical-notation"
                                                | "geo-shape" | "tabular-data" => {
                                                    datavalue_obj.as_str().map(String::from)
                                                }
                                                "wikibase-item" | "wikibase-property"
                                                | "wikibase-lexeme" | "wikibase-form"
                                                | "wikibase-sense" => {
                                                    datavalue_obj["id"].as_str().map(String::from)
                                                }
                                                "time" => {
                                                    datavalue_obj["time"].as_str().map(String::from)
                                                }
                                                "quantity" => {
                                                    if let Some(amount) = datavalue_obj["amount"].as_str() {
                                                        let unit_url = datavalue_obj["unit"].as_str().unwrap_or("1");
                                                        let unit_id = unit_url.split('/').last().unwrap_or("1");
                                                        let lower = datavalue_obj["lowerBound"].as_str();
                                                        let upper = datavalue_obj["upperBound"].as_str();

                                                        let mut val_str = amount.to_string();
                                                        if let (Some(l), Some(u)) = (lower, upper) {
                                                            val_str = format!("{}({},{})", val_str, l, u);
                                                        }
                                                        if unit_id != "1" {
                                                            val_str = format!("{}[{}]", val_str, unit_id);
                                                        }
                                                        Some(val_str)
                                                    } else {
                                                        None
                                                    }
                                                }
                                                "monolingualtext" => {
                                                    let text = datavalue_obj["text"].as_str();
                                                    let lang = datavalue_obj["language"].as_str();
                                                    if let (Some(t), Some(l)) = (text, lang) {
                                                        if languages_to_include.contains(l) {
                                                            Some(format!("{}@{}", t, l))
                                                        } else {
                                                            None
                                                        }
                                                    } else {
                                                        None
                                                    }
                                                }
                                                "globe-coordinate" => {
                                                    let lat = datavalue_obj["latitude"].as_f64();
                                                    let lon = datavalue_obj["longitude"].as_f64();
                                                    if let (Some(lat), Some(lon)) = (lat, lon) {
                                                        Some(format!("{},{}", lat, lon))
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => None,
                                            };

                                            if let Some(val) = extracted_value {
                                                if datatype == "quantity" {
                                                    grouped_claims.insert(prop_id.to_string(), vec![val.clone()]);
                                                } else {
                                                    grouped_claims.entry(prop_id.to_string()).or_insert_with(Vec::new).push(val.clone());
                                                }

                                                if property_datatypes_to_include_in_metadata.contains(&datatype.to_string()) {
                                                    export_claims.entry(prop_id.to_string()).or_insert_with(Vec::new).push(val);
                                                    *local_metrics.property_usage_count.entry(prop_id.to_string()).or_insert(0) += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let mut added_to_omni = false;
                        let mut added_to_globe = false;
                        let mut added_to_temporal = false;
                        let mut added_to_astro = false;

                        let extract_raw_num = |val: &str| -> f64 {
                            val.split('(').next().unwrap_or(val).split('[').next().unwrap_or(val).parse::<f64>().unwrap_or(0.0)
                        };

                        let is_omni_match = has_relevant_sitelink || has_included_concept;

                        if is_omni_match {
                            let mut tags = HashSet::new();
                            for p31 in &p31_qids {
                                if let Some(parents) = tag_dict_clone.get(p31) {
                                    for parent in parents {
                                        if omni_search_index_tags.contains(parent) {
                                            tags.insert(parent.clone());
                                            *local_metrics.tag_usage_in_omni.entry(parent.clone()).or_insert(0) += 1;
                                        }
                                    }
                                }
                            }
                            let tags_str = tags.into_iter().collect::<Vec<_>>().join(",");

                            let mut search_terms = HashSet::new();
                            if let Some(labels_obj) = parsed["labels"].as_object() {
                                for (lang, lang_data) in labels_obj {
                                    if languages_to_include.contains(lang) {
                                        if let Some(label) = lang_data["value"].as_str() {
                                            if !label.is_empty() { search_terms.insert(label.to_string()); }
                                        }
                                    }
                                }
                            }
                            if let Some(aliases_obj) = parsed["aliases"].as_object() {
                                for (lang, alias_list) in aliases_obj {
                                    if languages_to_include.contains(lang) {
                                        if let Some(arr) = alias_list.as_array() {
                                            for alias in arr {
                                                if let Some(alias_val) = alias["value"].as_str() {
                                                    if !alias_val.is_empty() { search_terms.insert(alias_val.to_string()); }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if !search_terms.is_empty() {
                                added_to_omni = true;
                                for term in &search_terms {
                                    if !omni_search_index_case_sensitive {
                                        let term_lower = term.to_lowercase();
                                        entity_data.omni_search_lines.push_str(&format!("{term_lower}{text_delimiter}{entity_id}{text_delimiter}{tags_str}\n"));
                                    } else {
                                        entity_data.omni_search_lines.push_str(&format!("{term}{text_delimiter}{entity_id}{text_delimiter}{tags_str}\n"));
                                    }
                                }

                                local_metrics.qids_used_in_omni_search += 1;
                                local_metrics.omni_search_entries_created += search_terms.len() as u64;
                                if has_relevant_sitelink {
                                    local_metrics.qids_used_in_omni_search_with_article_total += 1;
                                    for lang in &unique_langs {
                                        *local_metrics.qids_used_in_omni_search_by_lang.entry(lang.clone()).or_insert(0) += 1;
                                    }
                                } else {
                                    local_metrics.qids_used_in_omni_search_no_article_total += 1;
                                }
                            }
                        }

                        if create_globe_coordinate_search_index {
                            if include_all_matches_in_globe_coordinate_search_index || has_relevant_sitelink {
                                if let Some(coords) = grouped_claims.get("P625") {
                                    for coord_str in coords {
                                        let parts: Vec<&str> = coord_str.split(',').collect();
                                        if parts.len() == 2 {
                                            if let (Ok(lat), Ok(lon)) = (parts[0].parse::<f32>(), parts[1].parse::<f32>()) {
                                                let encoded_coord = encoding::safe_spatial_encode(lat, lon, earth_ctx);
                                                let mut coord_tags = std::collections::HashSet::new();
                                                for p31 in &p31_qids {
                                                    if let Some(parents) = tag_dict_clone.get(p31) {
                                                        for parent in parents {
                                                            if globe_coordinate_search_index_tags.contains(parent) {
                                                                coord_tags.insert(parent.clone());
                                                                *local_metrics.tag_usage_in_globe.entry(parent.clone()).or_insert(0) += 1;
                                                            }
                                                        }
                                                    }
                                                }
                                                let coord_tags_str = coord_tags.into_iter().collect::<Vec<_>>().join(",");
                                                entity_data.coordinate_lines.push_str(&format!("{encoded_coord}{text_delimiter}{entity_id}{text_delimiter}{coord_tags_str}\n"));
                                                added_to_globe = true;
                                            }
                                        }
                                    }
                                    if added_to_globe {
                                        local_metrics.qids_used_in_coordinate_search += 1;
                                        if has_relevant_sitelink {
                                            local_metrics.qids_used_in_coordinate_search_with_article_total += 1;
                                            for lang in &unique_langs {
                                                *local_metrics.qids_used_in_coordinate_search_by_lang.entry(lang.clone()).or_insert(0) += 1;
                                            }
                                        } else {
                                            local_metrics.qids_used_in_coordinate_search_without_article += 1;
                                        }
                                    }
                                }
                            }
                        }

                        if create_temporal_search_index {
                            if include_all_matches_in_temporal_search_index || has_relevant_sitelink {
                                for pid in ["P569", "P570", "P571", "P580", "P582"] {
                                    if let Some(times) = grouped_claims.get(pid) {
                                        for time_val in times {
                                            let timestamp = encoding::safe_temporal_encode(time_val.as_str());
                                            let mut temp_tags = std::collections::HashSet::new();
                                            for p31 in &p31_qids {
                                                if let Some(parents) = tag_dict_clone.get(p31) {
                                                    for parent in parents {
                                                        if temporal_search_index_tags.contains(parent) {
                                                            temp_tags.insert(parent.clone());
                                                            *local_metrics.tag_usage_in_temporal.entry(parent.clone()).or_insert(0) += 1;
                                                        }
                                                    }
                                                }
                                            }
                                            let temp_tags_str = temp_tags.into_iter().collect::<Vec<_>>().join(",");
                                            entity_data.temporal_lines.push_str(&format!("{timestamp}{text_delimiter}{entity_id}{text_delimiter}{pid}{text_delimiter}{temp_tags_str}\n"));
                                            added_to_temporal = true;
                                        }
                                    }
                                }
                                if added_to_temporal {
                                    local_metrics.qids_used_in_temporal_search += 1;
                                    if has_relevant_sitelink {
                                        local_metrics.qids_used_in_temporal_search_with_article_total += 1;
                                        for lang in &unique_langs {
                                            *local_metrics.qids_used_in_temporal_search_by_lang.entry(lang.clone()).or_insert(0) += 1;
                                        }
                                    } else {
                                        local_metrics.qids_used_in_temporal_search_without_article += 1;
                                    }
                                }
                            }
                        }

                        if create_astronomical_search_index && !matched_astro_concepts.is_empty() {
                            if include_all_matches_in_astronomical_search_index || has_relevant_sitelink {
                                let mut magnitude_ok = true;
                                if let Some(magnitudes) = grouped_claims.get("P1457") {
                                    for mag_str in magnitudes {
                                        if extract_raw_num(mag_str) > max_apparent_magnitude {
                                            magnitude_ok = false;
                                        }
                                    }
                                }
                                if magnitude_ok {
                                    let ra = grouped_claims.get("P6257").and_then(|v| v.first()).map_or(0.0, |v| extract_raw_num(v));
                                    let dec = grouped_claims.get("P6258").and_then(|v| v.first()).map_or(0.0, |v| extract_raw_num(v));
                                    let encoded_astro = encoding::safe_spatial_encode(dec as f32, ra as f32, celestial_ctx);
                                    let mut astro_tags = std::collections::HashSet::new();
                                    for p31 in &p31_qids {
                                        if let Some(parents) = tag_dict_clone.get(p31) {
                                            for parent in parents {
                                                if astronomical_search_index_tags.contains(parent) {
                                                    astro_tags.insert(parent.clone());
                                                    *local_metrics.tag_usage_in_astro.entry(parent.clone()).or_insert(0) += 1;
                                                }
                                            }
                                        }
                                    }
                                    let astro_tags_str = astro_tags.into_iter().collect::<Vec<_>>().join(",");

                                    entity_data.astronomical_lines.push_str(&format!(
                                        "{encoded_astro}{text_delimiter}{entity_id}{text_delimiter}{astro_tags_str}\n"
                                    ));

                                    added_to_astro = true;

                                    local_metrics.qids_used_in_astronomical_search += 1;
                                    if has_relevant_sitelink {
                                        local_metrics.qids_used_in_astronomical_search_with_article_total += 1;
                                        for lang in &unique_langs {
                                            *local_metrics.qids_used_in_astronomical_search_by_lang.entry(lang.clone()).or_insert(0) += 1;
                                        }
                                    } else {
                                        local_metrics.qids_used_in_astronomical_search_without_article += 1;
                                    }
                                }
                            }
                        }

                        let is_used_anywhere = added_to_omni || added_to_globe || added_to_temporal || added_to_astro;

                        if !is_used_anywhere {
                            local_metrics.num_lines_skipped += 1;
                            return Some((PreparedBatch::empty(), local_metrics));
                        }

                        for (lang, title) in valid_sitelinks {
                            entity_data.sitelinks_mapping_lines.push_str(&format!("{lang}{text_delimiter}{title}{text_delimiter}{entity_id}\n"));
                        }

                        let mut metadata_pairs = Vec::new();
                        for (pid, values) in export_claims {
                            metadata_pairs.push(format!("{}:{}", pid, values.join(";;")));
                        }

                        if !metadata_pairs.is_empty() {
                            entity_data.metadata_lines.push_str(&format!("{entity_id}{text_delimiter}{}\n",metadata_pairs.join(text_delimiter)));
                            local_metrics.metadata_entries_written += 1;
                        } else {
                            entity_data.metadata_lines.push_str(&format!("{entity_id}{text_delimiter}\n"));
                            local_metrics.empty_metadata_entries_written += 1;
                            local_metrics.metadata_entries_written += 1;
                        }

                        local_metrics.qids_used_total += 1;
                        return Some((entity_data, local_metrics));

                    } else if is_p_property {
                        local_metrics.pids_found += 1;
                        let datatype = parsed["datatype"].as_str().unwrap_or("unknown");

                        if !property_datatypes_to_include_in_metadata.contains(datatype) {
                            local_metrics.num_lines_skipped += 1;
                            return Some((PreparedBatch::empty(), local_metrics));
                        }

                        let mut entity_data = PreparedBatch::empty();

                        if let Some(labels_obj) = parsed["labels"].as_object() {
                            for (lang, lang_data) in labels_obj {
                                if languages_to_include.contains(lang) {
                                    let label = lang_data["value"].as_str().unwrap_or("");
                                    let description = parsed["descriptions"][lang]["value"]
                                        .as_str()
                                        .unwrap_or("");

                                    if !label.is_empty() {
                                        entity_data.properties_lines.push_str(&format!("{entity_id}{text_delimiter}{lang}{text_delimiter}{label}{text_delimiter}{description}{text_delimiter}{datatype}\n"));
                                    }
                                }
                            }
                        }

                        if entity_data.properties_lines.is_empty() {
                            local_metrics.num_lines_skipped += 1;
                            return Some((PreparedBatch::empty(), local_metrics));
                        }
                        local_metrics.pids_used += 1;
                        return Some((entity_data, local_metrics));
                    } else {
                        local_metrics.num_lines_skipped += 1;
                        return Some((PreparedBatch::empty(), local_metrics));
                    }
                })
                .reduce(
                    || (PreparedBatch::empty(), ParserMetrics::default()),
                    |(mut batch_a, mut metrics_a), (batch_b, metrics_b)| {
                        batch_a = batch_a.merge(batch_b);
                        metrics_a.merge(metrics_b);
                        (batch_a, metrics_a)
                    }
                );

            {
                let mut global_m = global_metrics_clone.lock().unwrap();
                global_m.num_lines_read += batch_len;
                global_m.merge(batch_metrics);
            }

            parsed_tx.send(processed_batch).unwrap();
        }
        drop(parsed_tx);
        println!("Main Thread: Finished. Waiting for Writer Thread to flush to disk...");
    });

    println!("Parsing complete! Starting final processing...");

    println!("Sorting sitelink mapping...");
    let _ = txt_file_processing::external_merge_sort(
        sitelinks_qid_mapping_raw_unsorted_txt_path
            .to_str()
            .unwrap(),
        sitelinks_qid_mapping_raw_txt_path.to_str().unwrap(),
        SortMode::Alphabetical,
        ram_limit_mb,
        num_threads,
        text_delimiter,
    );

    println!("Starting conversion of sitelinks_mapping.txt to fast lookup database...");
    if let Err(e) = txt_file_processing::build_sitelink_database(
        sitelinks_qid_mapping_raw_txt_path.to_str().unwrap(),
        sitelinks_qid_mapping_db_path.to_str().unwrap(),
        text_delimiter,
    ) {
        eprintln!("Error during database conversion: {}", e);
    } else {
        println!("Successfully converted sitelinks mapping to database.");
    }

    println!("Starting sorting of text files...");

    let _ = txt_file_processing::external_merge_sort(
        omni_search_unsorted_txt_path.to_str().unwrap(),
        omni_search_txt_path.to_str().unwrap(),
        SortMode::Alphabetical,
        ram_limit_mb,
        num_threads,
        text_delimiter,
    );

    let _ = txt_file_processing::external_merge_sort(
        properties_search_unsorted_txt_path.to_str().unwrap(),
        properties_search_txt_path.to_str().unwrap(),
        SortMode::XId,
        ram_limit_mb,
        num_threads,
        text_delimiter,
    );

    let _ = txt_file_processing::external_merge_sort(
        meta_data_unsorted_txt_path.to_str().unwrap(),
        meta_data_txt_path.to_str().unwrap(),
        SortMode::XId,
        ram_limit_mb,
        num_threads,
        text_delimiter,
    );

    if create_globe_coordinate_search_index {
        let _ = txt_file_processing::external_merge_sort(
            globe_coordinate_search_unsorted_txt_path.to_str().unwrap(),
            globe_coordinate_search_txt_path.to_str().unwrap(),
            SortMode::Numeric,
            ram_limit_mb,
            num_threads,
            text_delimiter,
        );
    }

    if create_temporal_search_index {
        let _ = txt_file_processing::external_merge_sort(
            temporal_search_unsorted_txt_path.to_str().unwrap(),
            temporal_search_txt_path.to_str().unwrap(),
            SortMode::Numeric,
            ram_limit_mb,
            num_threads,
            text_delimiter,
        );
    }

    if create_astronomical_search_index {
        let _ = txt_file_processing::external_merge_sort(
            astronomical_search_unsorted_txt_path.to_str().unwrap(),
            astronomical_search_txt_path.to_str().unwrap(),
            SortMode::Numeric,
            ram_limit_mb,
            num_threads,
            text_delimiter,
        );
    }
    println!("Sorting complete!");

    let m: ParserMetrics = Arc::try_unwrap(global_metrics)
        .expect("Other threads are still holding the Arc!")
        .into_inner()
        .expect("Mutex is bad!");

    let summary_text = m.make_summary();

    logs::write_summary_to_log(&summary_text, &settings, true, constants::PARSER_LOG)?;

    checkpoints::make_checkpoint(&settings, 1, "parser", None)
        .map_err(|e| format!("Finished parsing, but failed to create checkpoint: {}", e))?;

    Ok(())
}

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub database_content: DatabaseContent,
    pub urls: Urls,
    pub match_patterns: MatchPatterns,
    pub paths: Paths,
    pub performance: Performance,
    pub other: Other,
}

impl Settings {
    pub fn load_from_file(file_path: &str) -> Result<Settings, Box<dyn std::error::Error>> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name(file_path))
            .build()?;

        let settings: Settings = builder.try_deserialize()?;
        Ok(settings)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseContent {
    pub include_concepts_with_given_property_in_omni_search_index: Vec<String>,
    pub omni_search_index_tags: Vec<String>,
    pub omni_search_index_case_sensitive: bool,

    pub create_globe_coordinate_search_index: bool,
    pub include_all_matches_in_globe_coordinate_search_index: bool,
    pub globe_coordinate_search_index_tags: Vec<String>,

    pub create_temporal_search_index: bool,
    pub include_all_matches_in_temporal_search_index: bool,
    pub temporal_search_index_tags: Vec<String>,

    pub create_astronomical_search_index: bool,
    pub include_all_matches_in_astronomical_search_index: bool,
    pub astronomical_search_index_tags: Vec<String>,
    pub astronomical_objects_to_include: Vec<String>,

    pub max_apparent_magnitude: f64,
    pub property_datatypes_to_include_in_metadata: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Urls {
    pub wikidata_dump_url: String,
    pub wikipedia_base_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MatchPatterns {
    pub wikipedia_zim_file_match_pattern: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Paths {
    pub wikidata_dump_path: String,
    pub language_config_path: String,
    pub data_dir: String,
    pub log_dir: String,
    pub tmp_dir: String,
    pub bin_dir: String,
    pub checkpoint_dir: String,
    pub example_articles_dir: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Performance {
    pub thread_count: usize,
    pub read_buffer_size_kb: usize,
    pub write_buffer_size_kb: usize,
    pub ram_limit_mb: usize,
    pub zstd_dict_size_kb: usize,
    pub zstd_dict_training_sample_size_mb: usize,
    pub zstd_compression_level: i32,
    pub zstd_window_size_kb: usize,
    pub omni_search_index_term_encoding_bytes: usize,

    pub omni_search_sparse_index_ram_limit_kb: usize,
    pub globe_coordinate_search_index_ram_limit_kb: usize,
    pub temporal_serach_index_ram_limit_kb: usize,
    pub astronomical_search_index_ram_limit_kb: usize,

    pub omni_search_chunk_size_bytes: usize,
    pub globe_coordinate_search_chunk_size_bytes: usize,
    pub temporal_search_chunk_size_bytes: usize,
    pub astronomical_search_chunk_size_bytes: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Other {
    pub text_delimiter: String,
}

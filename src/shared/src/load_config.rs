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

#[derive(Deserialize, Debug, Clone)]
pub struct DatabaseContent {
    pub wikis_to_include: Vec<String>,
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
    pub wiki_base_url: String,
    pub wiktionary_base_url: String,
    pub wikiquote_base_url: String,
    pub wikisource_base_url: String,
    pub wikivoyage_base_url: String,
    pub wikinews_base_url: String,
    pub wikiversity_base_url: String,
    pub wikibooks_base_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MatchPatterns {
    pub wiki_zim_file_match_pattern: String,
    pub wiktionary_zim_file_match_pattern: String,
    pub wikiquote_zim_file_match_pattern: String,
    pub wikisource_zim_file_match_pattern: String,
    pub wikivoyage_zim_file_match_pattern: String,
    pub wikinews_zim_file_match_pattern: String,
    pub wikiversity_zim_file_match_pattern: String,
    pub wikibooks_zim_file_match_pattern: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Paths {
    // Download paths
    pub wikidata_dump_path: String,
    pub data_dir: String,
    // Temporary files for parsing
    pub omni_search_txt_file_path: String,
    pub properties_search_txt_file_path: String,
    pub globe_coordinate_search_txt_file_path: String,
    pub astronomical_search_txt_file_path: String,
    pub temporal_search_text_file_path: String,
    pub sitelinks_qid_mapping_txt_file_path: String,
    pub qid_index_txt_file_path: String,
    pub meta_data_txt_file_path: String,

    // Log files
    pub progression_log_file_path: String,

    // Binary files for the database
    pub content_bin_file_path: String,
    pub metadata_bin_file_path: String,
    pub omni_search_index_bin_file_path: String,
    pub globe_coordinate_search_index_bin_file_path: String,
    pub astronomical_search_index_bin_file_path: String,
    pub temporal_search_index_bin_file_path: String,
    pub properties_search_index_bin_file_path: String,
    pub content_pointers_bin_file_path: String,
    pub q_id_index_bin_file_path: String,
    // Other
    pub language_config_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Performance {
    pub thread_count: usize,
    pub buffer_size_kb: usize,
    pub ram_limit_mb: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Other {
    pub text_delimiter: String,
}

pub fn load_config_from_file(file_path: &str) -> Result<Settings, Box<dyn std::error::Error>> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name(file_path))
        .build()?;

    let settings: Settings = builder.try_deserialize()?;

    Ok(settings)
}

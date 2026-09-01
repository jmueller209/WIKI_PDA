use serde::Deserialize;

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Settings {
    pub database_content: DatabaseContent,
    pub urls: Urls,
    pub match_patterns: MatchPatterns,
    pub paths: Paths,
    pub performance: Performance,
    pub other: Other,
}

impl Settings {
    pub fn load_from_file(
        internal_file_path: &str,
    ) -> Result<Settings, Box<dyn std::error::Error>> {
        let internal_config = config::Config::builder()
            .add_source(config::File::with_name(internal_file_path))
            .build()?;

        let user_settings_path: String = internal_config
            .get_string("paths.user_settings_path")
            .unwrap_or_else(|_| "config/user_settings.toml".to_string());

        let builder = config::Config::builder()
            .add_source(config::File::with_name(&user_settings_path).required(true))
            .add_source(config::File::with_name(internal_file_path).required(true))
            .build()?;

        let settings: Settings = builder.try_deserialize()?;

        Ok(settings)
    }
}

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DatabaseContent {
    pub language_to_include: Vec<Language>,
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

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Urls {
    pub wikidata_dump_url: String,
    pub wikipedia_base_url: String,
}

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct MatchPatterns {
    pub wikipedia_zim_file_match_pattern: String,
}

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Paths {
    pub user_settings_path: String,
    pub wikidata_dump_path: String,
    pub data_dir: String,
    pub log_dir: String,
    pub tmp_dir: String,
    pub bin_dir: String,
    pub checkpoint_dir: String,
    pub example_articles_dir: String,
}

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Performance {
    pub thread_count: usize,
    pub read_buffer_size_kb: usize,
    pub write_buffer_size_kb: usize,
    pub ram_limit_mb: usize,
    pub zstd_dict_size_kb: usize,
    pub zstd_dict_training_sample_size_mb: usize,
    pub zstd_compression_level: i32,
    pub zstd_window_size_kb: usize,

    pub omni_search_sparse_index_ram_limit_kb: usize,
    pub globe_coordinate_search_index_ram_limit_kb: usize,
    pub temporal_search_index_ram_limit_kb: usize,
    pub astronomical_search_index_ram_limit_kb: usize,

    pub omni_search_chunk_size_bytes: usize,
    pub globe_coordinate_search_chunk_size_bytes: usize,
    pub temporal_search_chunk_size_bytes: usize,
    pub astronomical_search_chunk_size_bytes: usize,
}

#[derive(Default, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Other {
    pub text_delimiter: String,
    pub delete_source_binaries_after_merge: bool,
    pub delete_tmp_dir_after_generation: bool,
}

#[repr(u16)]
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    #[serde(rename = "en")]
    En = 1,
    #[serde(rename = "ceb")]
    Ceb = 2, // Cebuano (Very high article count due to bots)
    #[serde(rename = "de")]
    De = 3,
    #[serde(rename = "sv")]
    Sv = 4,
    #[serde(rename = "fr")]
    Fr = 5,
    #[serde(rename = "nl")]
    Nl = 6,
    #[serde(rename = "ru")]
    Ru = 7,
    #[serde(rename = "es")]
    Es = 8,
    #[serde(rename = "it")]
    It = 9,
    #[serde(rename = "pl")]
    Pl = 10,
    #[serde(rename = "ja")]
    Ja = 11,
    #[serde(rename = "zh")]
    Zh = 12, // Chinese
    #[serde(rename = "vi")]
    Vi = 13,
    #[serde(rename = "uk")]
    Uk = 14,
    #[serde(rename = "ar")]
    Ar = 15,
    #[serde(rename = "pt")]
    Pt = 16,
    #[serde(rename = "fa")]
    Fa = 17, // Persian
    #[serde(rename = "ca")]
    Ca = 18, // Catalan
    #[serde(rename = "sr")]
    Sr = 19, // Serbian
    #[serde(rename = "id")]
    Id = 20, // Indonesian
    #[serde(rename = "ko")]
    Ko = 21,
    #[serde(rename = "no")]
    No = 22, // Norwegian
    #[serde(rename = "fi")]
    Fi = 23,
    #[serde(rename = "tr")]
    Tr = 24,
    #[serde(rename = "hu")]
    Hu = 25,
    #[serde(rename = "cs")]
    Cs = 26, // Czech
    #[serde(rename = "ro")]
    Ro = 27,
    #[serde(rename = "eu")]
    Eu = 28, // Basque
    #[serde(rename = "ms")]
    Ms = 29, // Malay
    #[serde(rename = "eo")]
    Eo = 30, // Esperanto
    #[serde(rename = "he")]
    He = 31, // Hebrew
    #[serde(rename = "da")]
    Da = 32, // Danish
    #[serde(rename = "bg")]
    Bg = 33,
    #[serde(rename = "sk")]
    Sk = 34, // Slovak
    #[serde(rename = "et")]
    Et = 35, // Estonian
    #[serde(rename = "be")]
    Be = 36, // Belarusian
    #[serde(rename = "simple")]
    Simple = 37, // Simple English
    #[serde(rename = "el")]
    El = 38, // Greek
    #[serde(rename = "hr")]
    Hr = 39, // Croatian
    #[serde(rename = "lt")]
    Lt = 40, // Lithuanian
    #[serde(rename = "gl")]
    Gl = 41, // Galician
    #[serde(rename = "sl")]
    Sl = 42, // Slovenian
    #[serde(rename = "ur")]
    Ur = 43, // Urdu
    #[serde(rename = "hi")]
    Hi = 44, // Hindi
    #[serde(rename = "th")]
    Th = 45, // Thai
    #[serde(rename = "bn")]
    Bn = 46, // Bengali
    #[serde(rename = "ta")]
    Ta = 47, // Tamil
    #[serde(rename = "te")]
    Te = 48, // Telugu
    #[serde(rename = "sw")]
    Sw = 49, // Swahili
    #[serde(rename = "lv")]
    Lv = 50, // Latvian
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Ceb => "ceb",
            Language::De => "de",
            Language::Sv => "sv",
            Language::Fr => "fr",
            Language::Nl => "nl",
            Language::Ru => "ru",
            Language::Es => "es",
            Language::It => "it",
            Language::Pl => "pl",
            Language::Ja => "ja",
            Language::Zh => "zh",
            Language::Vi => "vi",
            Language::Uk => "uk",
            Language::Ar => "ar",
            Language::Pt => "pt",
            Language::Fa => "fa",
            Language::Ca => "ca",
            Language::Sr => "sr",
            Language::Id => "id",
            Language::Ko => "ko",
            Language::No => "no",
            Language::Fi => "fi",
            Language::Tr => "tr",
            Language::Hu => "hu",
            Language::Cs => "cs",
            Language::Ro => "ro",
            Language::Eu => "eu",
            Language::Ms => "ms",
            Language::Eo => "eo",
            Language::He => "he",
            Language::Da => "da",
            Language::Bg => "bg",
            Language::Sk => "sk",
            Language::Et => "et",
            Language::Be => "be",
            Language::Simple => "simple",
            Language::El => "el",
            Language::Hr => "hr",
            Language::Lt => "lt",
            Language::Gl => "gl",
            Language::Sl => "sl",
            Language::Ur => "ur",
            Language::Hi => "hi",
            Language::Th => "th",
            Language::Bn => "bn",
            Language::Ta => "ta",
            Language::Te => "te",
            Language::Sw => "sw",
            Language::Lv => "lv",
        }
    }
}

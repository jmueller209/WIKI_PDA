pub mod pipeline;
pub mod tools;
pub mod utils;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum RunMode {
    _00_Download,
    _01_WikidataParsing,
    _02_CompressionSetup,
    _03_ZimProcessing,
    _04_MetadataBinaryGeneration,
    _05_QID_IndexBinary,
    _06_PID_IndexBinary,
    _07_SearchIndexesBinary,
    _08_MergeBinaries,
    RestartAllClean,
    RestartAllPurge,
    ResumeAll,
    CleanAll,
    CleanAllExceptDownloads,
    ExtractSampleArticles,
    TestArticleProcessing,
    DebugArticleProcessingAnomalies,
    TestPipeline,
}

pub enum PipelineStep {
    CleanAll,
    Download,
    ParseWikiData,
    TrainDictionary,
    CreateContent,
    MakeIndexes,
}

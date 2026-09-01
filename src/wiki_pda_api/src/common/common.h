#ifndef WIKI_PDA_COMMON_H
#define WIKI_PDA_COMMON_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#include "../../lib/zstd/src/zstd.h"

#include "../../include/wiki_pda_types.h"
#include "../../include/wiki_pda_platforms.h"
#include "../../include/wiki_pda_options.h"
#include "../../lib/spatial_z/include/spatial_z.h"

#include "../indexes/omni_search.h"
#include "../indexes/globe_coordinate_search.h"
#include "../indexes/astronomical_search.h"
#include "../indexes/temporal_search.h"
#include "../indexes/pid_search.h"
#include "../indexes/qid_search.h"

#ifdef __cplusplus
extern "C" {
#endif

#ifdef DEBUG_MODE
    #define DEBUG_PRINT(fmt, ...) printf("[DEBUG] " fmt "\n", ##__VA_ARGS__)
#else
    #define DEBUG_PRINT(fmt, ...)
#endif

#define SUPPORT_MAJOR_VERSION 0
#define SUPPORT_MINOR_VERSION 1

// Changing this will break everything as this needs to match the generator
#define MAX_SPARSE_LEVELS 15
#define SD_SECTOR_SIZE 512
#define HEADER_SIZE_BYTES 4096
#define OMNI_SEARCH_INDEX_TERM_ENCODING_BYTES 24
#define MAGIC "WPDA"
#define DB_MAGIC_STRING ""
#define DB_MAGIC_LENGTH 0


typedef struct {
    uint8_t is_enabled;
    uint8_t num_sparse_levels;
    uint16_t _padding1;
    uint32_t top_level_rows;
    uint32_t term_size;
    uint32_t row_size;
    uint32_t chunk_size;
    uint32_t _padding2;
    uint64_t level_offsets[MAX_SPARSE_LEVELS];
    uint64_t level_sizes[MAX_SPARSE_LEVELS];
} IndexMetadata;

typedef struct {
    char magic[4];
    uint8_t version_major;
    uint8_t version_minor;
    uint8_t version_patch;
    uint8_t _padding;
    uint64_t offset_qid_hashmap;
    uint64_t offset_qid_index;
    uint64_t offset_titles;
    uint64_t offset_pid_hashmap;
    uint64_t offset_pid_index;
    uint64_t offset_pid_strings;
    uint64_t offset_content;
    uint64_t offset_metadata;
    uint64_t offset_zstd_dictionary;

    uint64_t size_qid_hashmap;
    uint64_t size_qid_index;
    uint64_t size_titles;
    uint64_t size_pid_hashmap;
    uint64_t size_pid_index;
    uint64_t size_pid_strings;
    uint64_t size_content;
    uint64_t size_metadata;
    uint64_t size_zstd_dictionary;

    IndexMetadata omni_search;
    IndexMetadata temporal_search;
    IndexMetadata astro_search;
    IndexMetadata globe_search;
} DatabaseHeader;

struct DatabaseContext_t {
#if WIKI_PDA_ENABLE_OMNI_SEARCH
    OmniSparseRow* omni_top_index;
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    AstronomicalSparseRow* astronomical_top_index;
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    TemporalSparseRow* temporal_top_index;
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    GlobeCoordinateSparseRow* globe_coordinate_top_index;
#endif

    uint8_t* zstd_dict;
    uint64_t zstd_dict_length;

    DatabaseHeader header;

    DatabasePlatform platform;
};

typedef struct {
    uint32_t qid;
    uint32_t tags;
    float distance;
    float lat;
    float lon;
} SpatialMatch;

typedef struct {
    MortonRange ranges[MAX_MORTON_RANGES];
    uint8_t num_ranges;
    uint8_t current_range_index;

    SpatialMatch sorted_results[MAX_SORTED_RESULTS];
    uint16_t num_sorted_results;
    uint16_t current_sorted_index;

    CompareCtx compare_ctx;
} SpatialCursorState;

typedef struct {
    char search_term[OMNI_SEARCH_TERM_SIZE];
    size_t term_length;
} OmniCursorState;

typedef struct {
    int64_t date_code;
    bool search_forward;
} TemporalCursorState;

typedef struct {
    uint64_t id;
    bool search_forward;
} IDCursorState;

struct SearchCursor_t {
    DatabaseContext* ctx;
    SearchQuery query;
    bool end_of_results;

    uint64_t next_read_offset;

    uint8_t raw_bytes[512];
    uint8_t current_row_index;
    uint8_t valid_rows_in_batch;
    size_t row_size;

    uint32_t seen_qids[MAX_DEDUPLICATION_CACHE];
    uint32_t seen_qid_count;

    char article_title_buffer[256];
    char match_term_buffer[256];

    union {
        SpatialCursorState spatial;
        OmniCursorState omni;
        TemporalCursorState temporal;
        IDCursorState id;
    } state;
};

typedef enum {
    ROW_MATCH,
    ROW_SKIP,
    ROW_JUMP,
    ROW_END
} RowEvalResult;

struct DataStream_t {
    DatabaseContext* ctx;
    uint64_t current_read_offset;
    uint32_t bytes_remaining_on_disk;
    ZSTD_DCtx* dctx;
    ZSTD_inBuffer input;
    uint8_t compressed_chunk[512];
    bool is_compressed;
};

#ifdef __cplusplus
}
#endif

#endif

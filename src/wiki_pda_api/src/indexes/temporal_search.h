#ifndef TEMPORAL_SEARCH_H
#define TEMPORAL_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} TemporalRow;

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t target_row;
    uint8_t _padding[4]; 
} TemporalSparseRow;

bool load_temporal_top_index(TemporalSparseRow** out_top_level_index, DatabasePlatform platform);

void free_temporal_top_index(TemporalSparseRow* top_level_index);

bool temporal_search(
    int64_t search_term,
    const TemporalSparseRow* top_level_ram_index, 
    uint64_t* out_abs_pointer, 
    DatabasePlatform platform
);

#endif

#endif // TEMPORAL_SEARCH_H

#ifndef GLOBE_COORDINATE_SEARCH_H
#define GLOBE_COORDINATE_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} GlobeCoordinateRow;

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t target_row;
    uint8_t _padding[4]; 
} GlobeCoordinateSparseRow;

bool load_globe_coordinate_top_index(GlobeCoordinateSparseRow** out_top_level_index, DatabasePlatform platform);

void free_globe_coordinate_top_index(GlobeCoordinateSparseRow* top_level_index);

bool globe_coordinate_search(
    uint64_t search_term,
    const GlobeCoordinateSparseRow* top_level_ram_index, 
    uint64_t* out_abs_pointer, 
    DatabasePlatform platform
);

#endif

#endif // GLOBE_COORDINATE_SEARCH_H

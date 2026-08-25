#ifndef ASTRONOMICAL_SEARCH_H
#define ASTRONOMICAL_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} AstronomicalRow;

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t target_row;
    uint8_t _padding[4]; 
} AstronomicalSparseRow;

bool load_astronomical_top_index(AstronomicalSparseRow** out_top_level_index, DatabasePlatform platform);

void free_astronomical_top_index(AstronomicalSparseRow* top_level_index);

bool astronomical_search(
    uint64_t search_term,
    const AstronomicalSparseRow* top_level_ram_index, 
    uint64_t* out_abs_pointer, 
    DatabasePlatform platform
);

#endif

#endif // ASTRONOMICAL_SEARCH_H

#ifndef OMNI_SEARCH_H
#define OMNI_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

#if WIKI_PDA_ENABLE_OMNI_SEARCH

typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t qid;
    uint32_t tags;
} OmniRow;

typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t target_row;
    uint8_t _padding[OMNI_SEARCH_TOTAL_ROW_SIZE - OMNI_SEARCH_TERM_SIZE - 4]; 
} OmniSparseRow;

bool load_omni_top_index(OmniSparseRow** out_top_level_index, DatabasePlatform platform);
void free_omni_top_index(OmniSparseRow* top_level_index);

bool omni_search(
    const char* search_term,
    const OmniSparseRow* top_level_index,
    uint64_t* out_abs_pointer,
    DatabasePlatform platform
);

#endif

#endif // OMNI_SEARCH_H

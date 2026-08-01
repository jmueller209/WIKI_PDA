#ifndef OMNI_SEARCH_H
#define OMNI_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "database_constants.h"
#include "database_io.h"

// The main row returned by the search (32 bytes)
typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t qid;
    uint32_t tags;
} OmniRow;

// The sparse row used for jumping through levels (Forced to 32 bytes)
typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t target_row;
    uint8_t _padding[OMNI_SEARCH_TOTAL_ROW_SIZE - OMNI_SEARCH_TERM_SIZE - 4]; 
} OmniSparseRow;

OmniSparseRow* load_top_level_index(void);
void free_top_level_index(OmniSparseRow* ram_index);

uint32_t omni_search(
    const char* target_query,
    const OmniSparseRow* top_level_ram_index,
    OmniRow* out_results,
    uint32_t max_results
);

#endif // OMNI_SEARCH_H

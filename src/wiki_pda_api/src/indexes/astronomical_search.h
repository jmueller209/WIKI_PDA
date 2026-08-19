#ifndef ASTRONOMICAL_SEARCH_H
#define ASTRONOMICAL_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

typedef struct __attribute__((packed)) {
    uint32_t term;
    uint32_t qid;
    uint32_t tags;
} AstronomicalRow;

typedef struct __attribute__((packed)) {
    uint32_t term;
    uint32_t target_row;
    uint8_t _padding[OMNI_SEARCH_TOTAL_ROW_SIZE - OMNI_SEARCH_TERM_SIZE - 4]; 
} AstronomicalSparseRow;

bool load_astronomical_top_index(AstronomicalSparseRow** out_top_level_index, DatabasePlatform platform);
void free_astronomical_top_index(AstronomicalSparseRow* top_level_index);

// bool omni_search(
//     const char* search_term,
//     const OmniSparseRow* top_level_ram_index,
//     uint64_t* out_abs_pointer
// );
//
// bool get_omni_row(
//     uint64_t abs_pointer,
//     uint32_t row_offset,
//     uint32_t num_rows,
//     OmniRow* out_row
// );

#endif // ASTRONOMICAL_SEARCH_H

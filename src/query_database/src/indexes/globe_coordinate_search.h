#ifndef GLOBE_COORDINATE_SEARCH_H
#define GLOBE_COORDINATE_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"

typedef struct __attribute__((packed)) {
    uint32_t term;
    uint32_t qid;
    uint32_t tags;
} GlobeCoordinateRow;

typedef struct __attribute__((packed)) {
    uint32_t term;
    uint32_t target_row;
    uint8_t _padding[OMNI_SEARCH_TOTAL_ROW_SIZE - OMNI_SEARCH_TERM_SIZE - 4]; 
} GlobeCoordinateSparseRow;

bool load_globe_coordinate_top_index(GlobeCoordinateSparseRow** out_top_level_index);
void free_globe_coordinate_top_index(GlobeCoordinateSparseRow* top_level_index);

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

#endif // GLOBE_COORDINATE_SEARCH_H

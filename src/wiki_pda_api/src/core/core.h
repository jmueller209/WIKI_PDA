#ifndef WIKI_PDA_CORE_H
#define WIKI_PDA_CORE_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/common.h" 
#include "../../lib/zstd/src/zstd.h"
#include "../../include/wiki_pda_platforms.h"

#ifdef __cplusplus
extern "C" {
#endif

void insert_sorted_spatial_match(
    SpatialCursorState* spatial,
    uint32_t qid,
    uint32_t tags,
    float distance,
    float lat,
    float lon,
    uint16_t max_results
);

bool search_next_id(SearchCursor* cursor, SearchResult* out_result);

bool search_next_in_index(SearchCursor* cursor, SearchResult* out_result);

bool load_and_verify_header(DatabaseContext* ctx);

bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length, DatabaseContext* ctx);

void free_zstd_dictionary(uint8_t* dictionary);

#ifdef __cplusplus
}
#endif

#endif // WIKI_PDA_CORE_H

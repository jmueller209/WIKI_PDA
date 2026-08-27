#include "../../include/wiki_pda_options.h"

#if WIKI_PDA_ENABLE_OMNI_SEARCH

#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "omni_search.h"
#include "../api/wiki_pda_internal.h"
#include "generic_search.h"

bool load_omni_top_index(OmniSparseRow** out_top_level_index, DatabasePlatform platform) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        OMNI_SEARCH_TOP_LEVEL_ROWS,
        sizeof(OmniSparseRow),
        OFFSETS_OMNI_SEARCH_LEVEL[OMNI_SEARCH_NUM_SPARSE_LEVELS],
        platform,
        "Omni"
    );

}

void free_omni_top_index(OmniSparseRow* top_level_index) {
    free_top_level_index_generic((void*) top_level_index);
}

bool omni_search(
    const char* target_query,
    const OmniSparseRow* top_level_ram_index,
    uint64_t* out_abs_pointer,
    DatabasePlatform platform
) {
    if (target_query == NULL || top_level_ram_index == NULL || out_abs_pointer == NULL) return false;
    if (OMNI_SEARCH_TOP_LEVEL_ROWS == 0) return false;

    size_t query_len = strlen(target_query);
    if (query_len > OMNI_SEARCH_TERM_SIZE) query_len = OMNI_SEARCH_TERM_SIZE;

    DEBUG_PRINT("=== STARTING SEARCH FOR: '%s' ===", target_query);

    uint32_t target_row = 0;
    uint32_t left = 0;
    uint32_t right = OMNI_SEARCH_TOP_LEVEL_ROWS - 1;
    uint32_t best_match = 0;
    uint32_t total_omni_rows = SIZES_OMNI_SEARCH_LEVEL[0] / sizeof(OmniRow);

    while (left <= right) {
        uint32_t mid = left + (right - left) / 2;
        int cmp = strncmp(top_level_ram_index[mid].term, target_query, OMNI_SEARCH_TERM_SIZE);
        if (cmp <= 0) { best_match = mid; left = mid + 1; } 
        else { if (mid == 0) break; right = mid - 1; }
    }
    target_row = top_level_ram_index[best_match].target_row;

    OmniSparseRow temp_sparse;
    for (int lvl = OMNI_SEARCH_NUM_SPARSE_LEVELS - 1; lvl >= 1; lvl--) {
        left = target_row;
        right = target_row + OMNI_SEARCH_CHUNK_SIZE_ROWS - 1;
        uint32_t total_sparse_rows = SIZES_OMNI_SEARCH_LEVEL[lvl] / sizeof(OmniSparseRow);
        if (right >= total_sparse_rows) {
            right = total_sparse_rows - 1;
        }
        best_match = left;

        while (left <= right) {
            uint32_t mid = left + (right - left) / 2;
            uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[lvl] + ((uint64_t)mid * sizeof(OmniSparseRow));

            if (!platform.read_fn(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow), platform.user_data)) {
                right = mid - 1; continue;
            }

            int cmp = strncmp(temp_sparse.term, target_query, OMNI_SEARCH_TERM_SIZE);
            if (cmp <= 0) { best_match = mid; left = mid + 1; } 
            else { if (mid == 0) break; right = mid - 1; }
        }

        uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[lvl] + ((uint64_t)best_match * sizeof(OmniSparseRow));
        if (!platform.read_fn(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow), platform.user_data)) {
            return false;
        }
        target_row = temp_sparse.target_row;
    }

    left = target_row;
    right = target_row + OMNI_SEARCH_CHUNK_SIZE_ROWS - 1;
    if (right >= total_omni_rows) {
        right = total_omni_rows - 1;
    }
    bool found = false;
    uint32_t first_match = left;
    OmniRow temp_row;

    while (left <= right) {
        uint32_t mid = left + (right - left) / 2;
        uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[0] + ((uint64_t)mid * sizeof(OmniRow));

        if (!platform.read_fn(byte_offset, (uint8_t*)&temp_row, sizeof(OmniRow), platform.user_data)) {
            right = mid - 1; continue;
        }

        int cmp = strncmp(temp_row.term, target_query, query_len);
        if (cmp >= 0) {
            if (cmp == 0) { found = true; first_match = mid; }
            if (mid == 0) break;
            right = mid - 1;
        } else { 
            left = mid + 1;
        }
    }

    if (!found) return false;

    *out_abs_pointer = OFFSETS_OMNI_SEARCH_LEVEL[0] + ((uint64_t)first_match * sizeof(OmniRow));
    return true;
}

#endif

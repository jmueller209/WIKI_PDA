#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "omni_search.h"

bool load_omni_top_index(OmniSparseRow** out_top_level_index, DatabasePlatform platform) {
    if (out_top_level_index == NULL) return false;
    if (OMNI_SEARCH_TOP_LEVEL_ROWS == 0) {
        *out_top_level_index = NULL;
        return false;
    }

    uint32_t total_bytes = OMNI_SEARCH_TOP_LEVEL_ROWS * sizeof(OmniSparseRow);
    OmniSparseRow* ram_index = (OmniSparseRow*)malloc(total_bytes);
    if (ram_index == NULL) {
        *out_top_level_index = NULL;
        return false; 
    }

    uint64_t top_offset = OFFSETS_OMNI_SEARCH_LEVEL[OMNI_SEARCH_NUM_SPARSE_LEVELS];

    if (!platform.read_fn(top_offset, (uint8_t*)ram_index, total_bytes, platform.user_data)) {
        free(ram_index);
        *out_top_level_index = NULL;
        return false;
    }

    #ifdef DEBUG_MODE
        printf("[DEBUG] Loaded Top-Level RAM Index (%u rows).\n", OMNI_SEARCH_TOP_LEVEL_ROWS);
    #endif

    *out_top_level_index = ram_index;
    return true;
}

void free_omni_top_index(OmniSparseRow* top_level_index) {
    if (top_level_index != NULL) {
        free(top_level_index);
    }
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

    #ifdef DEBUG_MODE
        printf("\n[DEBUG] === STARTING SEARCH FOR: '%s' ===\n", target_query);
    #endif

    uint32_t target_row = 0;
    uint32_t left = 0;
    uint32_t right = OMNI_SEARCH_TOP_LEVEL_ROWS - 1;
    uint32_t best_match = 0;

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

bool omni_row_passes_tags(const OmniRow* row, uint32_t exact_tags, uint32_t include_tags, uint32_t exclude_tags) {
    if (exact_tags != 0 && row->tags != exact_tags) {
        return false;
    }

    if (include_tags != 0 && (row->tags & include_tags) != include_tags) {
        return false;
    }

    if (exclude_tags != 0 && (row->tags & exclude_tags) != 0) {
        return false;
    }

    return true;
}

#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "omni_search.h"
#include "database_constants.h"

OmniSparseRow* load_top_level_index(void) {
    if (OMNI_SEARCH_TOP_LEVEL_ROWS == 0) return NULL;

    uint32_t total_bytes = OMNI_SEARCH_TOP_LEVEL_ROWS * sizeof(OmniSparseRow);
    OmniSparseRow* ram_index = (OmniSparseRow*)malloc(total_bytes);
    if (ram_index == NULL) return NULL; 

    uint64_t top_offset = OFFSETS_OMNI_SEARCH_LEVEL[OMNI_SEARCH_NUM_SPARSE_LEVELS];

    if (!platform_database_read(top_offset, (uint8_t*)ram_index, total_bytes)) {
        free(ram_index);
        return NULL;
    }

#ifdef DEBUG_MODE
    printf("[DEBUG] Loaded Top-Level RAM Index (%u rows).\n", OMNI_SEARCH_TOP_LEVEL_ROWS);
    uint32_t check_limit = OMNI_SEARCH_TOP_LEVEL_ROWS < 3 ? OMNI_SEARCH_TOP_LEVEL_ROWS : 3;
    for (uint32_t i = 0; i < check_limit; i++) {
        printf("[DEBUG] RAM Row %u: '%.*s' -> Row %u\n", 
               i, OMNI_SEARCH_TERM_SIZE, ram_index[i].term, ram_index[i].target_row);
    }
#endif

    return ram_index;
}

void free_top_level_index(OmniSparseRow* ram_index) {
    if (ram_index != NULL) free(ram_index);
}

uint32_t omni_search(
    const char* target_query,
    const OmniSparseRow* top_level_ram_index,
    OmniRow* out_results,
    uint32_t max_results
) {
    if (max_results == 0 || OMNI_SEARCH_TOP_LEVEL_ROWS == 0) return 0;

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

#ifdef DEBUG_MODE
    printf("[DEBUG] Phase 1 (RAM): Landed on '%.*s' -> Jump to Level %u, Row %u\n", 
           OMNI_SEARCH_TERM_SIZE, top_level_ram_index[best_match].term, 
           OMNI_SEARCH_NUM_SPARSE_LEVELS - 1, target_row);
#endif

    OmniSparseRow temp_sparse;
    for (int lvl = OMNI_SEARCH_NUM_SPARSE_LEVELS - 1; lvl >= 1; lvl--) {
        left = target_row;
        right = target_row + OMNI_SEARCH_CHUNK_SIZE_ROWS - 1;
        best_match = left;

        while (left <= right) {
            uint32_t mid = left + (right - left) / 2;

            uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[lvl] + ((uint64_t)mid * sizeof(OmniSparseRow));

            if (!platform_database_read(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow))) {
                right = mid - 1; continue;
            }

            int cmp = strncmp(temp_sparse.term, target_query, OMNI_SEARCH_TERM_SIZE);
            if (cmp <= 0) { best_match = mid; left = mid + 1; } 
            else { if (mid == 0) break; right = mid - 1; }
        }

        uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[lvl] + ((uint64_t)best_match * sizeof(OmniSparseRow));
        platform_database_read(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow));
        target_row = temp_sparse.target_row;

#ifdef DEBUG_MODE
        printf("[DEBUG] Phase 2 (Lvl %d): Landed on '%.*s' -> Jump to Row %u\n", 
               lvl, OMNI_SEARCH_TERM_SIZE, temp_sparse.term, target_row);
#endif
    }

    left = target_row;
    right = target_row + OMNI_SEARCH_CHUNK_SIZE_ROWS - 1;
    bool found = false;
    uint32_t first_match = left;
    OmniRow temp_row;

    while (left <= right) {
        uint32_t mid = left + (right - left) / 2;

        uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[0] + ((uint64_t)mid * sizeof(OmniRow));

        if (!platform_database_read(byte_offset, (uint8_t*)&temp_row, sizeof(OmniRow))) {
            right = mid - 1; continue;
        }

        int cmp = strncmp(temp_row.term, target_query, query_len);
        if (cmp >= 0) {
            if (cmp == 0) { found = true; first_match = mid; }
            if (mid == 0) break;
            right = mid - 1;
        } else { left = mid + 1; }
    }

    if (!found) return 0; 

    uint32_t results_count = 0;
    while (results_count < max_results) {
        uint64_t byte_offset = OFFSETS_OMNI_SEARCH_LEVEL[0] + ((uint64_t)(first_match + results_count) * sizeof(OmniRow));

        if (!platform_database_read(byte_offset, (uint8_t*)&temp_row, sizeof(OmniRow))) break; 

        if (strncmp(temp_row.term, target_query, query_len) == 0) {
            memcpy(&out_results[results_count], &temp_row, sizeof(OmniRow));
            results_count++;
        } else break; 
    }

    return results_count; 
}

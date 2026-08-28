#include "../../include/wiki_pda_options.h"

#if WIKI_PDA_ENABLE_OMNI_SEARCH

#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "omni_search.h"
#include "generic_search.h"
#include "../common/common.h"

bool load_omni_top_index(OmniSparseRow** out_top_level_index, DatabaseContext* ctx) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        ctx->header.omni_search.top_level_rows,
        sizeof(OmniSparseRow),
        ctx->header.omni_search.level_offsets[ctx->header.omni_search.num_sparse_levels],
        ctx->platform,
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
    DatabaseContext* ctx
) {
    if (target_query == NULL || top_level_ram_index == NULL || out_abs_pointer == NULL || ctx == NULL) return false;
    if (ctx->header.omni_search.top_level_rows == 0) return false;

    size_t query_len = strlen(target_query);
    if (query_len > ctx->header.omni_search.term_size) query_len = ctx->header.omni_search.term_size;

    DEBUG_PRINT("=== STARTING SEARCH FOR: '%s' ===", target_query);

    uint32_t target_row = 0;
    uint32_t left = 0;
    uint32_t right = ctx->header.omni_search.top_level_rows - 1;
    uint32_t best_match = 0;
    uint32_t total_omni_rows = ctx->header.omni_search.level_sizes[0] / sizeof(OmniRow);

    while (left <= right) {
        uint32_t mid = left + (right - left) / 2;
        int cmp = strncmp(top_level_ram_index[mid].term, target_query, ctx->header.omni_search.term_size);
        if (cmp <= 0) { best_match = mid; left = mid + 1; } 
        else { if (mid == 0) break; right = mid - 1; }
    }
    target_row = top_level_ram_index[best_match].target_row;

    OmniSparseRow temp_sparse;
    for (int lvl = ctx->header.omni_search.num_sparse_levels - 1; lvl >= 1; lvl--) {
        left = target_row;
        right = target_row + ctx->header.omni_search.chunk_size - 1;
        uint32_t total_sparse_rows = ctx->header.omni_search.level_sizes[lvl] / sizeof(OmniSparseRow);
        if (right >= total_sparse_rows) {
            right = total_sparse_rows - 1;
        }
        best_match = left;

        while (left <= right) {
            uint32_t mid = left + (right - left) / 2;
            uint64_t byte_offset = ctx->header.omni_search.level_offsets[lvl] + ((uint64_t)mid * sizeof(OmniSparseRow));

            if (!ctx->platform.read_fn(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow), ctx->platform.user_data)) {
                right = mid - 1; continue;
            }

            int cmp = strncmp(temp_sparse.term, target_query, ctx->header.omni_search.term_size);
            if (cmp <= 0) { best_match = mid; left = mid + 1; } 
            else { if (mid == 0) break; right = mid - 1; }
        }

        uint64_t byte_offset = ctx->header.omni_search.level_offsets[lvl] + ((uint64_t)best_match * sizeof(OmniSparseRow));
        if (!ctx->platform.read_fn(byte_offset, (uint8_t*)&temp_sparse, sizeof(OmniSparseRow), ctx->platform.user_data)) {
            return false;
        }
        target_row = temp_sparse.target_row;
    }

    left = target_row;
    right = target_row + ctx->header.omni_search.chunk_size - 1;
    if (right >= total_omni_rows) {
        right = total_omni_rows - 1;
    }
    bool found = false;
    uint32_t first_match = left;
    OmniRow temp_row;

    while (left <= right) {
        uint32_t mid = left + (right - left) / 2;
        uint64_t byte_offset = ctx->header.omni_search.level_offsets[0] + ((uint64_t)mid * sizeof(OmniRow));

        if (!ctx->platform.read_fn(byte_offset, (uint8_t*)&temp_row, sizeof(OmniRow), ctx->platform.user_data)) {
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

    *out_abs_pointer = ctx->header.omni_search.level_offsets[0] + ((uint64_t)first_match * sizeof(OmniRow));
    return true;
}

#endif

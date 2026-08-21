#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "../../include/database_platform.h"

bool load_top_level_index_generic(void** out_index,
                              uint32_t row_count,
                              size_t row_size,
                              uint64_t offset,
                              DatabasePlatform platform,
                              const char* index_name) {
    if (out_index == NULL) {
        return false;
    }
    *out_index = NULL;

    if (row_count == 0 || row_size == 0) {
        return false;
    }

    uint32_t total_bytes = row_count * (uint32_t)row_size;
    void* ram_index = malloc(total_bytes);
    if (ram_index == NULL) {
        return false;
    }

    if (!platform.read_fn(offset, (uint8_t*)ram_index, total_bytes, platform.user_data)) {
        free(ram_index);
        return false;
    }

#ifdef DEBUG_MODE
    printf("[DEBUG] Loaded Top-Level RAM Index [%s] (%u rows, %u bytes).\n",
           index_name ? index_name : "Generic",
           row_count,
           total_bytes);
#endif

    *out_index = ram_index;
    return true;
}


void free_top_level_index_generic(void* top_level_index) {
    if (top_level_index != NULL) {
        free(top_level_index);
    }
}

static inline int64_t read_term_safely_i64(const uint8_t* ptr) {
    int64_t term;
    memcpy(&term, ptr, sizeof(int64_t)); // Reads bytes 0-7 as signed
    return term;
}

static inline uint64_t read_term_safely_u64(const uint8_t* ptr) {
    uint64_t term;
    memcpy(&term, ptr, sizeof(uint64_t)); // Reads bytes 0-7 as unsigned
    return term;
}

static inline uint32_t read_row_safely(const uint8_t* ptr) {
    uint32_t row;
    memcpy(&row, ptr + 8, sizeof(uint32_t)); // Reads bytes 8-11
    return row;
}



#define GENERATE_SEARCH_FUNCTION(FUNC_NAME, KEY_TYPE, READ_TERM_FUNC) \
bool FUNC_NAME( \
    KEY_TYPE search_term, \
    const void* top_level_ram_index, \
    uint32_t top_level_rows, \
    uint32_t num_sparse_levels, \
    uint32_t chunk_size_rows, \
    const uint64_t* level_offsets, \
    size_t sparse_row_size, \
    size_t base_row_size, \
    uint64_t* out_abs_pointer, \
    const DatabasePlatform* platform \
) { \
    if (top_level_ram_index == NULL || out_abs_pointer == NULL || top_level_rows == 0) return false; \
\
    const uint8_t* ram_index_bytes = (const uint8_t*)top_level_ram_index; \
    uint32_t target_row = 0; \
    uint32_t left = 0; \
    uint32_t right = top_level_rows - 1; \
    uint32_t best_match = 0; \
\
    while (left <= right) { \
        uint32_t mid = left + (right - left) / 2; \
        const uint8_t* row_ptr = ram_index_bytes + (mid * sparse_row_size); \
        if (READ_TERM_FUNC(row_ptr) <= search_term) { \
            best_match = mid; left = mid + 1; \
        } else { \
            if (mid == 0) break; right = mid - 1; \
        } \
    } \
    const uint8_t* best_row_ptr = ram_index_bytes + (best_match * sparse_row_size); \
    target_row = read_row_safely(best_row_ptr); \
\
    size_t max_row_size = base_row_size > sparse_row_size ? base_row_size : sparse_row_size; \
    uint8_t temp_row_bytes[max_row_size]; \
\
    for (int lvl = num_sparse_levels - 1; lvl >= 1; lvl--) { \
        left = target_row; \
        right = target_row + chunk_size_rows - 1; \
        best_match = left; \
        while (left <= right) { \
            uint32_t mid = left + (right - left) / 2; \
            uint64_t byte_offset = level_offsets[lvl] + ((uint64_t)mid * sparse_row_size); \
            if (!platform->read_fn(byte_offset, temp_row_bytes, sparse_row_size, platform->user_data)) { \
                right = mid - 1; continue; \
            } \
            if (READ_TERM_FUNC(temp_row_bytes) <= search_term) { \
                best_match = mid; left = mid + 1; \
            } else { \
                if (mid == 0) break; right = mid - 1; \
            } \
        } \
        uint64_t byte_offset = level_offsets[lvl] + ((uint64_t)best_match * sparse_row_size); \
        if (!platform->read_fn(byte_offset, temp_row_bytes, sparse_row_size, platform->user_data)) return false; \
        target_row = read_row_safely(temp_row_bytes); \
    } \
\
    left = target_row; \
    right = target_row + chunk_size_rows - 1; \
    uint32_t first_match = UINT32_MAX; \
\
    while (left <= right) { \
        uint32_t mid = left + (right - left) / 2; \
        uint64_t byte_offset = level_offsets[0] + ((uint64_t)mid * base_row_size); \
        if (!platform->read_fn(byte_offset, temp_row_bytes, base_row_size, platform->user_data)) { \
            if (mid == 0) break; right = mid - 1; continue; \
        } \
        KEY_TYPE base_term = READ_TERM_FUNC(temp_row_bytes); \
        if (base_term >= search_term) { \
            first_match = mid; \
            if (mid == 0) break; right = mid - 1; \
        } else { \
            left = mid + 1; \
        } \
    } \
\
    if (first_match == UINT32_MAX) return false; \
    *out_abs_pointer = level_offsets[0] + ((uint64_t)first_match * base_row_size); \
    return true; \
}


GENERATE_SEARCH_FUNCTION(generic_int64_search, int64_t, read_term_safely_i64)


GENERATE_SEARCH_FUNCTION(generic_uint64_search, uint64_t, read_term_safely_u64)



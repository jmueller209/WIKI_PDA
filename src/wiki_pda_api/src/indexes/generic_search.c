#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <limits.h>
#include "../../include/wiki_pda_platforms.h"
#include "../../include/wiki_pda_options.h"

bool load_top_level_index_generic(
    void **out_index,
    uint32_t row_count,
    size_t row_size,
    uint64_t offset,
    DatabasePlatform platform,
    const char *index_name
) {
    if (out_index == NULL) {
        return false;
    }

    *out_index = NULL;

    if (row_count == 0 ||
        row_size == 0 ||
        platform.read_fn == NULL) {
        return false;
    }

    if ((size_t)row_count > SIZE_MAX / row_size) {
        return false;
    }

    const size_t total_bytes =
        (size_t)row_count * row_size;

    void *ram_index = malloc(total_bytes);

    if (ram_index == NULL) {
        return false;
    }

    if (!platform.read_fn(
            offset,
            (uint8_t *)ram_index,
            total_bytes,
            platform.user_data)) {

        free(ram_index);
        return false;
    }

#ifdef DEBUG_MODE
    printf(
        "[DEBUG] Loaded Top-Level RAM Index [%s] "
        "(%u rows, %zu bytes).\n",
        index_name ? index_name : "Generic",
        row_count,
        total_bytes
    );
#endif

    *out_index = ram_index;

    return true;
}


void free_top_level_index_generic(
    void *top_level_index
) {
    free(top_level_index);
}

static inline int64_t read_term_safely_i64(
    const uint8_t *ptr
) {
    int64_t value;

    memcpy(
        &value,
        ptr,
        sizeof(value)
    );

    return value;
}


static inline uint64_t read_term_safely_u64(
    const uint8_t *ptr
) {
    uint64_t value;

    memcpy(
        &value,
        ptr,
        sizeof(value)
    );

    return value;
}


static inline uint32_t read_row_safely(
    const uint8_t *ptr
) {
    uint32_t row;

    memcpy(
        &row,
        ptr + sizeof(uint64_t),
        sizeof(row)
    );

    return row;
}

static bool get_row_offset_checked(
    uint64_t section_offset,
    uint64_t section_size,
    uint64_t row_index,
    size_t row_size,
    uint64_t *out_offset
) {
    if (out_offset == NULL ||
        row_size == 0) {
        return false;
    }

    const uint64_t row_size_u64 =
        (uint64_t)row_size;

    if (section_size < row_size_u64) {
        return false;
    }

    if (section_offset >
        UINT64_MAX - section_size) {
        return false;
    }

    const uint64_t max_row_index =
        (section_size - row_size_u64) / row_size_u64;

    if (row_index > max_row_index) {
        return false;
    }

    const uint64_t row_offset =
        row_index * row_size_u64;

    if (section_offset >
        UINT64_MAX - row_offset) {
        return false;
    }

    *out_offset =
        section_offset + row_offset;

    return true;
}

static bool read_row_checked(
    DatabasePlatform platform,
    uint64_t section_offset,
    uint64_t section_size,
    uint64_t row_index,
    size_t row_size,
    uint8_t *buffer
) {
    if (platform.read_fn == NULL ||
        buffer == NULL ||
        row_size == 0) {
        return false;
    }

    uint64_t byte_offset;

    if (!get_row_offset_checked(
            section_offset,
            section_size,
            row_index,
            row_size,
            &byte_offset)) {
        return false;
    }

    return platform.read_fn(
        byte_offset,
        buffer,
        row_size,
        platform.user_data
    );
}

#define GENERATE_SEARCH_FUNCTION(FUNC_NAME, KEY_TYPE, READ_TERM_FUNC) \
bool FUNC_NAME( \
    KEY_TYPE search_term, \
    const void *top_level_ram_index, \
    uint32_t top_level_rows, \
    uint32_t num_sparse_levels, \
    uint32_t chunk_size_rows, \
    const uint64_t *level_offsets, \
    const uint64_t *level_sizes, \
    size_t sparse_row_size, \
    size_t base_row_size, \
    uint64_t *out_abs_pointer, \
    DatabasePlatform platform \
) \
{ \
    if (top_level_ram_index == NULL || \
        out_abs_pointer == NULL || \
        top_level_rows == 0 || \
        num_sparse_levels == 0 || \
        chunk_size_rows == 0 || \
        level_offsets == NULL || \
        level_sizes == NULL || \
        sparse_row_size == 0 || \
        base_row_size == 0 || \
        platform.read_fn == NULL) { \
        return false; \
    } \
\
    if (sparse_row_size < sizeof(uint64_t) + sizeof(uint32_t) || \
        base_row_size < sizeof(KEY_TYPE)) { \
        return false; \
    } \
\
    if ((size_t)top_level_rows > \
        SIZE_MAX / sparse_row_size) { \
        return false; \
    } \
\
    const uint8_t *ram_index_bytes = \
        (const uint8_t *)top_level_ram_index; \
\
    uint64_t target_row; \
\
    { \
        uint64_t lo = 0; \
        uint64_t hi = top_level_rows; \
\
        while (lo < hi) { \
            const uint64_t mid = \
                lo + (hi - lo) / 2; \
\
            const uint8_t *row_ptr = \
                ram_index_bytes + \
                (size_t)mid * sparse_row_size; \
\
            const KEY_TYPE key = \
                READ_TERM_FUNC(row_ptr); \
\
            if (key <= search_term) { \
                lo = mid + 1; \
            } else { \
                hi = mid; \
            } \
        } \
\
        if (lo == 0) { \
            target_row = \
                (uint64_t)read_row_safely(ram_index_bytes); \
        } else { \
            const uint64_t best = lo - 1; \
\
            const uint8_t *row_ptr = \
                ram_index_bytes + \
                (size_t)best * sparse_row_size; \
\
            target_row = \
                (uint64_t)read_row_safely(row_ptr); \
        } \
    } \
\
    const size_t max_row_size = \
        base_row_size > sparse_row_size \
            ? base_row_size \
            : sparse_row_size; \
\
    uint8_t *temp_row_bytes = \
        (uint8_t *)malloc(max_row_size); \
\
    if (temp_row_bytes == NULL) { \
        return false; \
    } \
\
    for (uint32_t level = num_sparse_levels; \
         level > 1; \
         --level) { \
\
        const uint32_t lvl = level - 1; \
\
        const uint64_t section_offset = \
            level_offsets[lvl]; \
\
        const uint64_t section_size = \
            level_sizes[lvl]; \
\
        if (section_size == 0 || \
            section_size % sparse_row_size != 0) { \
            free(temp_row_bytes); \
            return false; \
        } \
\
        const uint64_t total_rows = \
            section_size / sparse_row_size; \
\
        if (total_rows == 0 || \
            target_row >= total_rows) { \
            free(temp_row_bytes); \
            return false; \
        } \
\
        const uint64_t chunk_begin = \
            target_row; \
\
        const uint64_t remaining = \
            total_rows - chunk_begin; \
\
        const uint64_t chunk_end = \
            ((uint64_t)chunk_size_rows >= remaining) \
                ? total_rows \
                : chunk_begin + chunk_size_rows; \
\
        if (chunk_begin >= chunk_end) { \
            free(temp_row_bytes); \
            return false; \
        } \
\
        uint64_t lo = chunk_begin; \
        uint64_t hi = chunk_end; \
\
        while (lo < hi) { \
            const uint64_t mid = \
                lo + (hi - lo) / 2; \
\
            if (!read_row_checked( \
                    platform, \
                    section_offset, \
                    section_size, \
                    mid, \
                    sparse_row_size, \
                    temp_row_bytes)) { \
\
                free(temp_row_bytes); \
                return false; \
            } \
\
            const KEY_TYPE key = \
                READ_TERM_FUNC(temp_row_bytes); \
\
            if (key <= search_term) { \
                lo = mid + 1; \
            } else { \
                hi = mid; \
            } \
        } \
\
        uint64_t selected_row; \
\
        if (lo == chunk_begin) { \
            selected_row = chunk_begin; \
        } else { \
            selected_row = lo - 1; \
        } \
\
        if (!read_row_checked( \
                platform, \
                section_offset, \
                section_size, \
                selected_row, \
                sparse_row_size, \
                temp_row_bytes)) { \
\
            free(temp_row_bytes); \
            return false; \
        } \
\
        target_row = \
            (uint64_t)read_row_safely(temp_row_bytes); \
    } \
\
    const uint64_t base_offset = \
        level_offsets[0]; \
\
    const uint64_t base_size = \
        level_sizes[0]; \
\
    if (base_size == 0 || \
        base_size % base_row_size != 0) { \
        free(temp_row_bytes); \
        return false; \
    } \
\
    const uint64_t total_base_rows = \
        base_size / base_row_size; \
\
    if (total_base_rows == 0 || \
        target_row >= total_base_rows) { \
        free(temp_row_bytes); \
        return false; \
    } \
\
    const uint64_t chunk_begin = \
        target_row; \
\
    const uint64_t remaining = \
        total_base_rows - chunk_begin; \
\
    const uint64_t chunk_end = \
        ((uint64_t)chunk_size_rows >= remaining) \
            ? total_base_rows \
            : chunk_begin + chunk_size_rows; \
\
    if (chunk_begin >= chunk_end) { \
        free(temp_row_bytes); \
        return false; \
    } \
\
    uint64_t lo = chunk_begin; \
    uint64_t hi = chunk_end; \
\
    while (lo < hi) { \
        const uint64_t mid = \
            lo + (hi - lo) / 2; \
\
        if (!read_row_checked( \
                platform, \
                base_offset, \
                base_size, \
                mid, \
                base_row_size, \
                temp_row_bytes)) { \
\
            free(temp_row_bytes); \
            return false; \
        } \
\
        const KEY_TYPE key = \
            READ_TERM_FUNC(temp_row_bytes); \
\
        if (key < search_term) { \
            lo = mid + 1; \
        } else { \
            hi = mid; \
        } \
    } \
\
    uint64_t first_match = lo; \
\
    if (first_match >= chunk_end) { \
        if (chunk_end >= total_base_rows) { \
            free(temp_row_bytes); \
            return false; \
        } \
\
        first_match = chunk_end; \
    } \
\
    if (first_match >= total_base_rows) { \
        free(temp_row_bytes); \
        return false; \
    } \
\
    if (!read_row_checked( \
            platform, \
            base_offset, \
            base_size, \
            first_match, \
            base_row_size, \
            temp_row_bytes)) { \
\
        free(temp_row_bytes); \
        return false; \
    } \
\
    const KEY_TYPE final_key = \
        READ_TERM_FUNC(temp_row_bytes); \
\
    if (final_key < search_term) { \
        free(temp_row_bytes); \
        return false; \
    } \
\
    uint64_t final_offset; \
\
    if (!get_row_offset_checked( \
            base_offset, \
            base_size, \
            first_match, \
            base_row_size, \
            &final_offset)) { \
\
        free(temp_row_bytes); \
        return false; \
    } \
\
    *out_abs_pointer = final_offset; \
\
    free(temp_row_bytes); \
    return true; \
}


GENERATE_SEARCH_FUNCTION(
    generic_int64_search,
    int64_t,
    read_term_safely_i64
)

GENERATE_SEARCH_FUNCTION(
    generic_uint64_search,
    uint64_t,
    read_term_safely_u64
)

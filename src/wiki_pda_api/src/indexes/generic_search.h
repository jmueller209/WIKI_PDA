#ifndef GENERIC_SEARCH_H
#define GENERIC_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../../include/wiki_pda_platforms.h"

#ifdef __cplusplus
extern "C" {
#endif

bool load_top_level_index_generic(void** out_index,
                              uint32_t row_count,
                              size_t row_size,
                              uint64_t offset,
                              DatabasePlatform platform,
                              const char* index_name);

void free_top_level_index_generic(void* top_level_index);

bool generic_int64_search(
    int64_t search_term,
    const void* top_level_ram_index,
    uint32_t top_level_rows,
    uint32_t num_sparse_levels,
    uint32_t chunk_size_rows,
    const uint64_t* level_offsets,
    const uint64_t* level_sizes,
    size_t sparse_row_size,
    size_t base_row_size,
    uint64_t* out_abs_pointer,
    DatabasePlatform platform
);

bool generic_uint64_search(
    uint64_t search_term,
    const void* top_level_ram_index,
    uint32_t top_level_rows,
    uint32_t num_sparse_levels,
    uint32_t chunk_size_rows,
    const uint64_t* level_offsets,
    const uint64_t* level_sizes,
    size_t sparse_row_size,
    size_t base_row_size,
    uint64_t* out_abs_pointer,
    DatabasePlatform platform
);

#ifdef __cplusplus
}
#endif

#endif // GENERIC_SEARCH_H

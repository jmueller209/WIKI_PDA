#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

#include "../../include/database_platform.h"

/**
 * Generic loader for sparse top-level RAM indexes.
 *
 * @param out_index    Pointer to destination pointer (will be set to NULL on failure).
 * @param row_count    Number of rows to load.
 * @param row_size     Size of a single row (sizeof(RowType)).
 * @param offset       Byte offset on disk/partition to read from.
 * @param platform     DatabasePlatform containing read_fn and user_data.
 * @param index_name   Optional label used for debug logs (can be NULL).
 * @return true on success, false on failure.
 */
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

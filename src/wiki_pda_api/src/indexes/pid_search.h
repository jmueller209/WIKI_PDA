#ifndef PID_SEARCH_H
#define PID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

/**
 * @brief O(1) Lookup Table for PIDs.
 * Address = (PID - 1) * 6 Bytes.
 */
typedef struct __attribute__((packed)) {
    uint32_t start_index; /**< Row in pid_index.bin where the translations start */
    uint16_t entry_count; /**< Number of available languages (rows) for this PID. 0 = PID does not exist. */
} PIDHashMapRow; // Exactly 6 Bytes

/**
 * @brief The actual entries for each language (pid_index.bin).
 * The hashmap dictates how many of these rows we need to read consecutively.
 */
typedef struct __attribute__((packed)) {
    uint16_t project_id;      /**< Maps to the global project_id (e.g., 2=dewiki) */
    uint32_t title_offset;    /**< Relative offset in pid_strings.bin for the name */
    uint32_t desc_offset;     /**< Relative offset in pid_strings.bin for the description */
} PIDIndexRow; // Exactly 10 Bytes


bool get_property_index_data(uint32_t pid, uint16_t lang_id, uint32_t* out_title_offset, uint32_t* out_desc_offset, DatabasePlatform platform);

bool get_property_title(uint32_t title_offset, char* out_title, size_t max_length, DatabasePlatform platform);

bool get_property_desc(uint32_t descr_offset, char* out_descr, size_t max_length, DatabasePlatform platform);

#endif // PID_SEARCH_H

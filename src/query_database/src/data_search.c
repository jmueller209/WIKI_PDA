#include "data_search.h"
#include "database_constants.h"
#include <stdlib.h>
#include <stdbool.h>
#include <stdint.h>

bool get_data(uint64_t data_offset, uint32_t data_length, uint8_t** out_data, bool is_metadata) {
    if (out_data == NULL || data_length == 0) {
        return false;
    }

    uint64_t base_offset = is_metadata ? OFFSETS_METADATA : OFFSETS_CONTENT;
    uint64_t byte_offset = data_offset + base_offset;

    uint8_t* buffer = (uint8_t*)malloc(data_length);
    if (buffer == NULL) {
        return false;
    }

    if (!platform_database_read(byte_offset, buffer, data_length)) {
        free(buffer);
        return false;
    }

    *out_data = buffer;
    return true;
}

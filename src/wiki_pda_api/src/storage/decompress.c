#include "../common/generated_database_constants.h"
#include "../../lib/zstd/src/zstd.h"
#include "../../include/database_platform.h"
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>


bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length, DatabasePlatform platform) {
    if (out_dictionary == NULL || out_length == NULL) {
        return false;
    }

    uint64_t dict_size = SIZES_ZSTD_DICTIONARY;
    uint64_t dict_offset = OFFSETS_ZSTD_DICTIONARY;

    if (dict_size == 0) {
        return false;
    }

    uint8_t* buffer = (uint8_t*)malloc(dict_size);
    if (buffer == NULL) {
        return false;
    }

    if (!platform.read_fn(dict_offset, buffer, dict_size, platform.user_data)) {
        free(buffer);
        return false;
    }

    *out_dictionary = buffer;
    *out_length = dict_size;

    return true;
}

void free_zstd_dictionary(uint8_t* dict) {
    if (dict != NULL) {
        free(dict);
    }
}


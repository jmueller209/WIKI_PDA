#include "../common/generated_database_constants.h"
#include <stdint.h>
#include <stdbool.h>
#include "../../lib/zstd/src/zstd.h"
#include "../../include/database_platform.h"
bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length, DatabasePlatform platform);

void free_zstd_dictionary(uint8_t* dictionary);

bool decompress_data(
    const uint8_t* compressed_data,
    uint32_t compressed_length,
    const uint8_t* dictionary,
    uint64_t dict_length,
    char** out_decompressed_text,
    uint32_t* out_decompressed_length
);

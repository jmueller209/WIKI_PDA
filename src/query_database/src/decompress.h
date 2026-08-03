#include "data_search.h"
#include "database_constants.h"
#include <stdint.h>
#include <stdbool.h>
#include <zstd.h> 
bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length);
bool decompress_data(
    const uint8_t* compressed_data,
    uint32_t compressed_length,
    const uint8_t* dictionary,
    uint64_t dict_length,
    char** out_decompressed_text,
    uint32_t* out_decompressed_length
);

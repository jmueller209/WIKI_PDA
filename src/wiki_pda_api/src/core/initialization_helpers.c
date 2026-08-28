#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

#include "../common/common.h"

bool load_and_verify_header(DatabaseContext* ctx) {
    if (!ctx->platform.read_fn(0, (uint8_t*)&(ctx->header), sizeof(DatabaseHeader), ctx->platform.user_data)) {
        DEBUG_PRINT("INIT FAILED: Could not read header block.\n");
        return false;
    }

    if (memcmp(ctx->header.magic, MAGIC, 4) != 0) {
        DEBUG_PRINT("INIT FAILED: Invalid magic bytes.\n");
        return false;
    }

    if (ctx->header.version != WIKI_PDA_SUPPORTED_DB_VERSION) {
        DEBUG_PRINT("INIT FAILED: Version mismatch. Expected v%d, got v%u.\n", 
                    WIKI_PDA_SUPPORTED_DB_VERSION, ctx->header.version);
        return false;
    }

    return true;
}

bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length, DatabaseContext* ctx) {
    if (out_dictionary == NULL || out_length == NULL) {
        return false;
    }

    uint64_t dict_size = ctx->header.size_zstd_dictionary;
    uint64_t dict_offset = ctx->header.offset_zstd_dictionary;

    if (dict_size == 0) {
        return false;
    }

    uint8_t* buffer = (uint8_t*)malloc(dict_size);
    if (buffer == NULL) {
        return false;
    }

    if (!ctx->platform.read_fn(dict_offset, buffer, dict_size, ctx->platform.user_data)) {
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

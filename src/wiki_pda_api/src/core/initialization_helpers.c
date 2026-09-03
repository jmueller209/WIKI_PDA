#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <inttypes.h>

#include "../common/common.h"

bool load_and_verify_header(DatabaseContext* ctx) {
    if (!ctx->platform.read_fn(0, (uint8_t*)&(ctx->header), sizeof(DatabaseHeader), ctx->platform.user_data)) {
        DEBUG_PRINT("INIT FAILED: Could not read header block.\n");
        return false;
    }

    if (memcmp(ctx->header.magic, WPDA_MAGIC, WPDA_MAGIC_LENGTH) != 0) {
        DEBUG_PRINT("INIT FAILED: Invalid magic bytes.\n");
        return false;
    }

    if (ctx->header.version_major != SUPPORT_MAJOR_VERSION || 
        ctx->header.version_minor != SUPPORT_MINOR_VERSION) {
        DEBUG_PRINT("INIT FAILED: Version mismatch. Expected v%d.%d.x, got v%u.%u.%u.\n", 
                    SUPPORT_MAJOR_VERSION, SUPPORT_MINOR_VERSION, 
                    ctx->header.version_major, ctx->header.version_minor, ctx->header.version_patch);
        return false;
    }

    DEBUG_PRINT("\n=======================================================\n");
    DEBUG_PRINT("===           DATABASE HEADER SUCCESSFULLY LOADED   ===\n");
    DEBUG_PRINT("=======================================================\n");
    DEBUG_PRINT("Magic   : %c%c%c%c\n", ctx->header.magic[0], ctx->header.magic[1], ctx->header.magic[2], ctx->header.magic[3]);

    DEBUG_PRINT("Version : %u.%u.%u\n", ctx->header.version_major, ctx->header.version_minor, ctx->header.version_patch);

    DEBUG_PRINT("\n--- Core Offsets & Sizes ---\n");
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "QID HashMap",    ctx->header.offset_qid_hashmap,     ctx->header.size_qid_hashmap);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "QID Index",      ctx->header.offset_qid_index,       ctx->header.size_qid_index);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "Titles",         ctx->header.offset_titles,          ctx->header.size_titles);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "PID HashMap",    ctx->header.offset_pid_hashmap,     ctx->header.size_pid_hashmap);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "PID Index",      ctx->header.offset_pid_index,       ctx->header.size_pid_index);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "PID Strings",    ctx->header.offset_pid_strings,     ctx->header.size_pid_strings);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "Content",        ctx->header.offset_content,         ctx->header.size_content);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "Metadata",       ctx->header.offset_metadata,        ctx->header.size_metadata);
    DEBUG_PRINT("%-18s | Offset: %12" PRIu64 " | Size: %12" PRIu64 "\n", "ZSTD Dictionary", ctx->header.offset_zstd_dictionary, ctx->header.size_zstd_dictionary);

    #define PRINT_INDEX_META(name, meta) \
        DEBUG_PRINT("\n--- Index: %s ---\n", name); \
        DEBUG_PRINT("Enabled          : %s\n", (meta).is_enabled ? "TRUE" : "FALSE"); \
        if ((meta).is_enabled) { \
            DEBUG_PRINT("Num Sparse Levels: %u\n", (meta).num_sparse_levels); \
            DEBUG_PRINT("Top Level Rows   : %" PRIu32 "\n", (meta).top_level_rows); \
            DEBUG_PRINT("Term Size        : %" PRIu32 "\n", (meta).term_size); \
            DEBUG_PRINT("Row Size         : %" PRIu32 "\n", (meta).row_size); \
            DEBUG_PRINT("Chunk Size (Rows): %" PRIu32 "\n", (meta).chunk_size); \
            DEBUG_PRINT("Level Layout     :\n"); \
            for (int i = 0; i <= (meta).num_sparse_levels; i++) { \
                DEBUG_PRINT("  -> Level %d: Offset = %10" PRIu64 " | Size = %10" PRIu64 "\n", \
                            i, (meta).level_offsets[i], (meta).level_sizes[i]); \
            } \
        }

    PRINT_INDEX_META("Omni Search",         ctx->header.omni_search);
    PRINT_INDEX_META("Temporal Search",     ctx->header.temporal_search);
    PRINT_INDEX_META("Astronomical Search", ctx->header.astro_search);
    PRINT_INDEX_META("Globe Search",        ctx->header.globe_search);

    DEBUG_PRINT("=======================================================\n\n");

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

#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include "../../lib/zstd/src/zstd.h"
#include "wiki_pda_internal.h"
#include "../common/generated_database_constants.h"

struct DataStream_t {
    DatabaseContext* ctx;
    uint64_t current_read_offset;
    uint32_t bytes_remaining_on_disk;
    ZSTD_DCtx* dctx;
    ZSTD_inBuffer input;
    uint8_t compressed_chunk[512]; 
    bool is_compressed;
};

DataStream* data_stream_begin(DatabaseContext* ctx, uint64_t data_offset, uint32_t data_length) {
    DEBUG_PRINT("data_stream_begin called. ctx=%p, offset=%llu, len=%u", 
                 (void*)ctx, (unsigned long long)data_offset, data_length);

    if (ctx == NULL || data_length == 0) {
        DEBUG_PRINT("FAILED: ctx is NULL or data_length is 0");
        return NULL;
    }

    bool is_content = (data_offset >= OFFSETS_CONTENT) && 
                      (data_offset + data_length <= OFFSETS_CONTENT + SIZES_CONTENT);

    bool is_metadata = (data_offset >= OFFSETS_METADATA) && 
                       (data_offset + data_length <= OFFSETS_METADATA + SIZES_METADATA);

    if (!is_content && !is_metadata) {
        DEBUG_PRINT("FAILED: Read offset %llu (len %u) is outside allowed regions or spans bounds.",
                    (unsigned long long)data_offset, data_length);
        return NULL;
    }

    DataStream* stream = (DataStream*)calloc(1, sizeof(struct DataStream_t));
    if (stream == NULL) {
        DEBUG_PRINT("FAILED: calloc returned NULL");
        return NULL;
    }

    stream->ctx = ctx;
    stream->is_compressed = is_content;

    if (stream->is_compressed) {
        if (data_length <= 4) {
            DEBUG_PRINT("FAILED: compressed data_length <= 4");
            free(stream);
            return NULL;
        }
        stream->current_read_offset = data_offset + 4;
        stream->bytes_remaining_on_disk = data_length - 4;
        DEBUG_PRINT("Adjusted offset (skipped header): offset=%llu, remaining=%u", 
                     (unsigned long long)stream->current_read_offset, stream->bytes_remaining_on_disk);

        stream->dctx = ZSTD_createDCtx();
        if (stream->dctx == NULL) {
            DEBUG_PRINT("FAILED: ZSTD_createDCtx returned NULL");
            free(stream);
            return NULL;
        }

        ZSTD_DCtx_reset(stream->dctx, ZSTD_reset_session_only);

        DEBUG_PRINT("Checking dictionary: ptr=%p, length=%llu", 
                     (void*)ctx->zstd_dict, (unsigned long long)ctx->zstd_dict_length);
        if (ctx->zstd_dict != NULL && ctx->zstd_dict_length > 0) {
            DEBUG_PRINT("Loading ZSTD dictionary...");
            ZSTD_DCtx_loadDictionary(stream->dctx, ctx->zstd_dict, ctx->zstd_dict_length);
            DEBUG_PRINT("ZSTD dictionary loaded.");
        }

        stream->input.src = stream->compressed_chunk;
        stream->input.size = 0;
        stream->input.pos = 0;
    } else {
        stream->current_read_offset = data_offset;
        stream->bytes_remaining_on_disk = data_length;
        stream->dctx = NULL;
        DEBUG_PRINT("Uncompressed stream initialized: offset=%llu, remaining=%u", 
                     (unsigned long long)stream->current_read_offset, stream->bytes_remaining_on_disk);
    }

    DEBUG_PRINT("data_stream_begin completed successfully.");
    return stream;
}

bool data_stream_read(DataStream* stream, char* out_buffer, uint32_t buffer_capacity, uint32_t* out_bytes_read) {
    if (stream == NULL || out_buffer == NULL || buffer_capacity == 0 || out_bytes_read == NULL) {
        DEBUG_PRINT("READ FAILED: Invalid arguments passed to data_stream_read");
        return false;
    }

    *out_bytes_read = 0;

    if (!stream->is_compressed) {
        if (stream->bytes_remaining_on_disk == 0) {
            DEBUG_PRINT("Read finished: No more bytes on disk.");
            return false; 
        }

        uint32_t to_read = buffer_capacity;
        if (to_read > stream->bytes_remaining_on_disk) {
            to_read = stream->bytes_remaining_on_disk;
        }

        DEBUG_PRINT("Disk Read (Uncompressed): Calling read_fn(offset=%llu, size=%u)...", 
                     (unsigned long long)stream->current_read_offset, to_read);
        if (!stream->ctx->platform.read_fn(stream->current_read_offset, 
                                           (uint8_t*)out_buffer, 
                                           to_read, 
                                           stream->ctx->platform.user_data)) {
            DEBUG_PRINT("FAILED: platform.read_fn returned false!");
            return false;
        }

        stream->current_read_offset += to_read;
        stream->bytes_remaining_on_disk -= to_read;
        *out_bytes_read = to_read;

        DEBUG_PRINT("Read block complete. Produced %u bytes.", *out_bytes_read);
        return true;
    }

    ZSTD_outBuffer output = {
        .dst = out_buffer,
        .size = buffer_capacity,
        .pos = 0
    };

    while (output.pos == 0) {
        if (stream->input.pos >= stream->input.size) {
            if (stream->bytes_remaining_on_disk == 0) {
                DEBUG_PRINT("Read finished: No more bytes on disk.");
                return false; 
            }
            uint32_t to_read = sizeof(stream->compressed_chunk);
            if (to_read > stream->bytes_remaining_on_disk) {
                to_read = stream->bytes_remaining_on_disk;
            }

            DEBUG_PRINT("Disk Read (Compressed): Calling read_fn(offset=%llu, size=%u)...",
                         (unsigned long long)stream->current_read_offset, to_read);
            if (!stream->ctx->platform.read_fn(stream->current_read_offset,
                                               stream->compressed_chunk,
                                               to_read,
                                               stream->ctx->platform.user_data)) {
                DEBUG_PRINT("FAILED: platform.read_fn returned false!");
                return false;
            }
            DEBUG_PRINT("Disk Read: Success.");

            stream->current_read_offset += to_read;
            stream->bytes_remaining_on_disk -= to_read;

            stream->input.src = stream->compressed_chunk;
            stream->input.size = to_read;
            stream->input.pos = 0;
        }

        DEBUG_PRINT("ZSTD: Calling decompressStream (input_size=%zu, input_pos=%zu)...", 
                     stream->input.size, stream->input.pos);
        size_t const ret = ZSTD_decompressStream(stream->dctx, &output, &stream->input);
        if (ZSTD_isError(ret)) {
            DEBUG_PRINT("FAILED: ZSTD Error: %s", ZSTD_getErrorName(ret));
            return false; 
        }
        DEBUG_PRINT("ZSTD: decompressStream returned %zu, output_pos=%zu", ret, output.pos);

        if (ret == 0 && output.pos == 0 && stream->bytes_remaining_on_disk == 0 && stream->input.pos >= stream->input.size) {
            DEBUG_PRINT("ZSTD: Reached end of frame with no new output.");
            return false;
        }
    }

    *out_bytes_read = (uint32_t)output.pos;
    DEBUG_PRINT("Read block complete. Produced %u bytes.", *out_bytes_read);
    return true; 
}

bool data_stream_end(DataStream* stream) {
    if (stream == NULL) {
        DEBUG_PRINT("END FAILED: stream is NULL");
        return false;
    }
    DEBUG_PRINT("data_stream_end called. Freeing context.");

    if (stream->dctx != NULL) {
        ZSTD_freeDCtx(stream->dctx);
    }

    free(stream);
    DEBUG_PRINT("Stream closed successfully.");
    return true;
}

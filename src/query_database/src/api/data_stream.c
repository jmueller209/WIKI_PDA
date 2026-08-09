#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <zstd.h>

#include "wiki_db_internal.h"

#ifdef DEBUG_MODE
    #include <stdio.h>
    #define STREAM_DEBUG(fmt, ...) printf("[STREAM DEBUG] " fmt "\n", ##__VA_ARGS__)
#else
    #define STREAM_DEBUG(fmt, ...)
#endif

struct DataStream_t {
    DatabaseContext* ctx;
    uint64_t current_read_offset;
    uint32_t bytes_remaining_on_disk;
    ZSTD_DCtx* dctx;
    ZSTD_inBuffer input;
    uint8_t compressed_chunk[512]; 
};

DataStream* data_stream_begin(DatabaseContext* ctx, uint64_t data_offset, uint32_t data_length) {
    STREAM_DEBUG("data_stream_begin called. ctx=%p, offset=%llu, len=%u", 
                 (void*)ctx, (unsigned long long)data_offset, data_length);

    if (ctx == NULL || data_length <= 4) {
        STREAM_DEBUG("FAILED: ctx is NULL or data_length <= 4");
        return NULL;
    }

    DataStream* stream = (DataStream*)calloc(1, sizeof(struct DataStream_t));
    if (stream == NULL) {
        STREAM_DEBUG("FAILED: calloc returned NULL");
        return NULL;
    }

    stream->ctx = ctx;
    stream->current_read_offset = data_offset + 4;
    stream->bytes_remaining_on_disk = data_length - 4;
    STREAM_DEBUG("Adjusted offset (skipped header): offset=%llu, remaining=%u", 
                 (unsigned long long)stream->current_read_offset, stream->bytes_remaining_on_disk);

    stream->dctx = ZSTD_createDCtx();
    if (stream->dctx == NULL) {
        STREAM_DEBUG("FAILED: ZSTD_createDCtx returned NULL");
        free(stream);
        return NULL;
    }

    ZSTD_DCtx_reset(stream->dctx, ZSTD_reset_session_only);

    STREAM_DEBUG("Checking dictionary: ptr=%p, length=%llu", 
                 (void*)ctx->zstd_dict, (unsigned long long)ctx->zstd_dict_length);
    if (ctx->zstd_dict != NULL && ctx->zstd_dict_length > 0) {
        STREAM_DEBUG("Loading ZSTD dictionary...");
        ZSTD_DCtx_loadDictionary(stream->dctx, ctx->zstd_dict, ctx->zstd_dict_length);
        STREAM_DEBUG("ZSTD dictionary loaded.");
    }

    stream->input.src = stream->compressed_chunk;
    stream->input.size = 0;
    stream->input.pos = 0;

    STREAM_DEBUG("data_stream_begin completed successfully.");
    return stream;
}

bool data_stream_read(DataStream* stream, char* out_buffer, uint32_t buffer_capacity, uint32_t* out_bytes_read) {
    if (stream == NULL || out_buffer == NULL || buffer_capacity == 0 || out_bytes_read == NULL) {
        STREAM_DEBUG("READ FAILED: Invalid arguments passed to data_stream_read");
        return false;
    }

    *out_bytes_read = 0;

    ZSTD_outBuffer output = {
        .dst = out_buffer,
        .size = buffer_capacity,
        .pos = 0
    };

    while (output.pos == 0) {
        if (stream->input.pos >= stream->input.size) {
            if (stream->bytes_remaining_on_disk == 0) {
                STREAM_DEBUG("Read finished: No more bytes on disk.");
                return false; 
            }
            uint32_t to_read = sizeof(stream->compressed_chunk);
            if (to_read > stream->bytes_remaining_on_disk) {
                to_read = stream->bytes_remaining_on_disk;
            }

            STREAM_DEBUG("Disk Read: Calling read_fn(offset=%llu, size=%u)...", 
                         (unsigned long long)stream->current_read_offset, to_read);
            STREAM_DEBUG("Checking pointers: read_fn=%p, user_data=%p", 
                (void*)stream->ctx->platform.read_fn, 
                stream->ctx->platform.user_data);
            if (!stream->ctx->platform.read_fn(stream->current_read_offset, 
                                               stream->compressed_chunk, 
                                               to_read, 
                                               stream->ctx->platform.user_data)) {
                STREAM_DEBUG("FAILED: platform.read_fn returned false!");
                return false; 
            }
            STREAM_DEBUG("Disk Read: Success.");

            stream->current_read_offset += to_read;
            stream->bytes_remaining_on_disk -= to_read;

            stream->input.src = stream->compressed_chunk;
            stream->input.size = to_read;
            stream->input.pos = 0;
        }

        STREAM_DEBUG("ZSTD: Calling decompressStream (input_size=%zu, input_pos=%zu)...", 
                     stream->input.size, stream->input.pos);
        size_t const ret = ZSTD_decompressStream(stream->dctx, &output, &stream->input);
        if (ZSTD_isError(ret)) {
            STREAM_DEBUG("FAILED: ZSTD Error: %s", ZSTD_getErrorName(ret));
            return false; 
        }
        STREAM_DEBUG("ZSTD: decompressStream returned %zu, output_pos=%zu", ret, output.pos);

        if (ret == 0 && output.pos == 0 && stream->bytes_remaining_on_disk == 0 && stream->input.pos >= stream->input.size) {
            STREAM_DEBUG("ZSTD: Reached end of frame with no new output.");
            return false;
        }
    }

    *out_bytes_read = (uint32_t)output.pos;
    STREAM_DEBUG("Read block complete. Produced %u bytes.", *out_bytes_read);
    return true; 
}

bool data_stream_end(DataStream* stream) {
    if (stream == NULL) {
        STREAM_DEBUG("END FAILED: stream is NULL");
        return false;
    }
    STREAM_DEBUG("data_stream_end called. Freeing context.");
    if (stream->dctx != NULL) {
        ZSTD_freeDCtx(stream->dctx);
    }
    free(stream);
    STREAM_DEBUG("Stream closed successfully.");
    return true;
}

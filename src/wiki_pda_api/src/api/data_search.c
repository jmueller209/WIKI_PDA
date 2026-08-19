#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#include "../../include/search_types.h"
#include "../../include/database_platform.h"
#include "../common/generated_database_constants.h"
#include "../common/database_customizable_constants.h"
#include "../indexes/qid_search.h"
#include "../indexes/omni_search.h"
#include "../indexes/astronomical_search.h"
#include "../storage/decompress.h"
#include "../indexes/temporal_search.h"
#include "../indexes/globe_coordinate_search.h"
#include "wiki_pda_internal.h"

#ifdef DEBUG_MODE
    #define API_DEBUG(fmt, ...) printf("[API DEBUG] " fmt "\n", ##__VA_ARGS__)
#else
    #define API_DEBUG(fmt, ...)
#endif

struct SearchCursor_t {
    DatabaseContext* ctx;
    SearchQuery query;
    char cached_search_term[OMNI_SEARCH_TERM_SIZE];
    size_t cached_term_length;

    uint64_t next_read_offset;
    bool end_of_results;

    union {
        OmniRow omni[16];
        uint8_t raw_bytes[512];
    } row_batch;

    uint8_t current_row_index;
    uint8_t valid_rows_in_batch;
    uint32_t seen_qids[MAX_DEDUPLICATION_CACHE];
    uint16_t seen_qid_count;
};

bool db_end(DatabaseContext* ctx) {
    API_DEBUG("db_end called.");
    if (ctx == NULL) return false;

    if (ctx->zstd_dict != NULL) free_zstd_dictionary(ctx->zstd_dict);
    if (ctx->omni_top_index != NULL) free_omni_top_index(ctx->omni_top_index); 
    if (ctx->astronomical_top_index != NULL) free_astronomical_top_index(ctx->astronomical_top_index);
    if (ctx->temporal_top_index != NULL) free_temporal_top_index(ctx->temporal_top_index);
    if (ctx->globe_coordinate_top_index != NULL) free_globe_coordinate_top_index(ctx->globe_coordinate_top_index);

    free(ctx);
    API_DEBUG("db_end finished successfully.");
    return true;
}

DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform) {
    API_DEBUG("db_init called.");
    if (platform.read_fn == NULL) {
        API_DEBUG("FAILED: platform.read_fn is NULL");
        return NULL;
    }

    DatabaseContext* ctx = (DatabaseContext*)calloc(1, sizeof(struct DatabaseContext_t));
    if (ctx == NULL) {
        API_DEBUG("FAILED: calloc failed for DatabaseContext");
        return NULL;
    }

    ctx->platform = platform;
    API_DEBUG("Platform saved. read_fn ptr: %p", (void*)ctx->platform.read_fn);

    ctx->omni_top_index = NULL;
    ctx->astronomical_top_index = NULL; 
    ctx->temporal_top_index = NULL;
    ctx->globe_coordinate_top_index = NULL;
    ctx->zstd_dict = NULL;
    ctx->zstd_dict_length = 0;

    API_DEBUG("Loading ZSTD dictionary...");
    if (!load_zstd_dictionary(&(ctx->zstd_dict), &(ctx->zstd_dict_length), ctx->platform)) {
        API_DEBUG("Error: Could not load ZSTD dictionary.");
        goto cleanup_and_fail;
    }

    API_DEBUG("Loading requested indexes...");
    if ((indexes_to_load & INDEX_OMNI) && !load_omni_top_index(&(ctx->omni_top_index), ctx->platform)) goto cleanup_and_fail;
    if ((indexes_to_load & INDEX_ASTRONOMICAL) && !load_astronomical_top_index(&(ctx->astronomical_top_index), ctx->platform)) goto cleanup_and_fail;
    if ((indexes_to_load & INDEX_TEMPORAL) && !load_temporal_top_index(&(ctx->temporal_top_index), ctx->platform)) goto cleanup_and_fail;
    if ((indexes_to_load & INDEX_GLOBE_COORDINATE) && !load_globe_coordinate_top_index(&(ctx->globe_coordinate_top_index), ctx->platform)) goto cleanup_and_fail;

    API_DEBUG("db_init completed successfully.");
    return ctx;

cleanup_and_fail:
    db_end(ctx); 
    return NULL;
}

SearchCursor* search_begin(DatabaseContext* ctx, const SearchQuery* query) {
    API_DEBUG("search_begin called.");
    if (ctx == NULL || query == NULL) return NULL;

    API_DEBUG("CTX sanity check: read_fn=%p", (void*)ctx->platform.read_fn);

    struct SearchCursor_t* cursor = (struct SearchCursor_t*)calloc(1, sizeof(struct SearchCursor_t));
    if (cursor == NULL) return NULL;

    cursor->ctx = ctx;
    cursor->query = *query; 
    cursor->end_of_results = false;
    cursor->current_row_index = 0;
    cursor->valid_rows_in_batch = 0;
    cursor->next_read_offset = 0;
    cursor->seen_qid_count = 0;

    switch (query->type) {
        case SEARCH_TYPE_OMNI:
            if (ctx->omni_top_index == NULL) {
                free(cursor);
                return NULL; 
            }
            strncpy(cursor->cached_search_term, query->target.term, OMNI_SEARCH_TERM_SIZE);
            cursor->cached_search_term[OMNI_SEARCH_TERM_SIZE - 1] = '\0';
            cursor->cached_term_length = strlen(cursor->cached_search_term);

            API_DEBUG("Searching Omni for: '%s'", cursor->cached_search_term);

            if (!omni_search(cursor->cached_search_term, ctx->omni_top_index, &cursor->next_read_offset, ctx->platform)) {
                API_DEBUG("No top index match found.");
                cursor->end_of_results = true; 
            }
            break;

        case SEARCH_TYPE_QID:
            break;

        default:
            free(cursor);
            return NULL;
    }

    API_DEBUG("search_begin successful.");
    return (SearchCursor*)cursor;
}

bool search_next(SearchCursor* cursor, SearchResult* out_result) {
    if (cursor == NULL || cursor->end_of_results) return false;

    while (true) {
        if (cursor->current_row_index >= cursor->valid_rows_in_batch) {
            API_DEBUG("Fetching new row batch at offset %llu. read_fn=%p", 
                     (unsigned long long)cursor->next_read_offset, (void*)cursor->ctx->platform.read_fn);

            if (!cursor->ctx->platform.read_fn(cursor->next_read_offset, 
                                   cursor->row_batch.raw_bytes, 
                                   512, 
                                   cursor->ctx->platform.user_data)) {
                cursor->end_of_results = true;
                return false;
            }
            cursor->next_read_offset += 512;
            cursor->current_row_index = 0;
            cursor->valid_rows_in_batch = 16;
        }

        OmniRow* row = &cursor->row_batch.omni[cursor->current_row_index++];

        if (strncmp(row->term, cursor->cached_search_term, cursor->cached_term_length) != 0) {
            cursor->end_of_results = true;
            return false;
        }

        if (!omni_row_passes_tags(row, cursor->query.exact_tags, cursor->query.include_tags, cursor->query.exclude_tags)) {
            continue;
        }

        uint64_t relative_data_offset = 0;
        uint32_t data_length = 0;

        if (!get_relative_data_offset_and_length(row->qid, 
                                        (uint16_t)cursor->query.article_type, 
                                        &relative_data_offset, 
                                        &data_length, 
                                        cursor->ctx->platform)) {
            continue;
        } 

        bool is_duplicate = false;
        uint16_t items_to_check = (cursor->seen_qid_count < MAX_DEDUPLICATION_CACHE) 
                                ? cursor->seen_qid_count 
                                : MAX_DEDUPLICATION_CACHE;

        for (uint16_t i = 0; i < items_to_check; i++) {
            if (cursor->seen_qids[i] == row->qid) {
                is_duplicate = true;
                break;
            }
        }

        if (is_duplicate) continue;

        cursor->seen_qids[(cursor->seen_qid_count++) % MAX_DEDUPLICATION_CACHE] = row->qid;

        out_result->qid = row->qid;
        out_result->tags = row->tags;
        out_result->article_type = cursor->query.article_type;
        out_result->title = row->term; 
        out_result->data_length = data_length;

        if (out_result->article_type == 0) {
            out_result->data_offset = relative_data_offset + OFFSETS_METADATA;
        } else {
            out_result->data_offset = relative_data_offset + OFFSETS_CONTENT;
        }

        API_DEBUG("Found match! QID=%u, read_fn is still: %p", out_result->qid, (void*)cursor->ctx->platform.read_fn);
        return true;
    }
}

bool search_end(SearchCursor* cursor) {
    API_DEBUG("search_end called.");
    if (cursor == NULL) return false;
    API_DEBUG("Freeing cursor. CTX read_fn before free: %p", (void*)cursor->ctx->platform.read_fn);
    free(cursor);
    API_DEBUG("Cursor freed.");
    return true;
}

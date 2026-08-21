#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#include "wiki_pda_internal.h"
#include "../common/generated_database_constants.h"
#include "../common/database_customizable_constants.h"
#include "../indexes/qid_search.h"
#include "../indexes/omni_search.h"
#include "../indexes/astronomical_search.h"
#include "../storage/decompress.h"
#include "../indexes/temporal_search.h"
#include "../indexes/globe_coordinate_search.h"
#include "../../include/search_types.h"
#include "../../include/database_platform.h"
#include "../../lib/tempus/include/tempus.h"
#include "../../lib/spatial_z/include/spatial_z.h"

struct SearchCursor_t {
    DatabaseContext* ctx;
    SearchQuery query;
    union {
        char omni_search_term[OMNI_SEARCH_TERM_SIZE];
        uint64_t globe_coordinate_search_term;
        uint64_t astronomical_search_term;
        int64_t temporal_search_term;
    } target;
    size_t cached_term_length; 
    uint64_t next_read_offset;
    bool end_of_results;
    uint8_t raw_bytes[512];
    uint8_t current_row_index;
    uint8_t valid_rows_in_batch;
    size_t row_size;

    uint32_t seen_qids[MAX_DEDUPLICATION_CACHE];
    uint16_t seen_qid_count;

    char current_title_buffer[256]; 
};

bool _check_tags(uint32_t row_tags, SearchTagMask exact_tags, SearchTagMask include_tags, SearchTagMask exclude_tags) {
    if (exact_tags != 0 && row_tags != exact_tags) return false;
    if (include_tags != 0 && (row_tags & include_tags) != include_tags) return false;
    if (exclude_tags != 0 && (row_tags & exclude_tags) != 0) return false;
    return true;
}

void fetch_real_title(uint32_t qid, char* buffer, size_t max_len, DatabasePlatform platform) {
    // Placeholder until metadata parsing is fully implemented
    strncpy(buffer, "Untitled", max_len - 1);
    buffer[max_len - 1] = '\0';
}

bool db_end(DatabaseContext* ctx) {
    DEBUG_PRINT("db_end called.");
    if (ctx == NULL) return false;

    if (ctx->zstd_dict != NULL) free_zstd_dictionary(ctx->zstd_dict);

#if WIKI_PDA_ENABLE_OMNI_SEARCH
    if (ctx->omni_top_index != NULL) free_omni_top_index(ctx->omni_top_index); 
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if (ctx->astronomical_top_index != NULL) free_astronomical_top_index(ctx->astronomical_top_index);
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    if (ctx->temporal_top_index != NULL) free_temporal_top_index(ctx->temporal_top_index);
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if (ctx->globe_coordinate_top_index != NULL) free_globe_coordinate_top_index(ctx->globe_coordinate_top_index);
#endif

    free(ctx);
    DEBUG_PRINT("db_end finished successfully.");
    return true;
}

DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform) {
    DEBUG_PRINT("db_init called.");
    if (platform.read_fn == NULL) return NULL;

    DatabaseContext* ctx = (DatabaseContext*)calloc(1, sizeof(struct DatabaseContext_t));
    if (ctx == NULL) return NULL;

    ctx->platform = platform;

#if WIKI_PDA_ENABLE_OMNI_SEARCH
    ctx->omni_top_index = NULL;
#endif
#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    ctx->astronomical_top_index = NULL; 
#endif
#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    ctx->temporal_top_index = NULL;
#endif
#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    ctx->globe_coordinate_top_index = NULL;
#endif

    ctx->zstd_dict = NULL;
    ctx->zstd_dict_length = 0;

    if (!load_zstd_dictionary(&(ctx->zstd_dict), &(ctx->zstd_dict_length), ctx->platform)) {
        goto cleanup_and_fail;
    }

#if WIKI_PDA_ENABLE_OMNI_SEARCH
    if ((indexes_to_load & INDEX_OMNI) && !load_omni_top_index(&(ctx->omni_top_index), ctx->platform)) goto cleanup_and_fail;
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if ((indexes_to_load & INDEX_ASTRONOMICAL) && !load_astronomical_top_index(&(ctx->astronomical_top_index), ctx->platform)) goto cleanup_and_fail;
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    if ((indexes_to_load & INDEX_TEMPORAL) && !load_temporal_top_index(&(ctx->temporal_top_index), ctx->platform)) goto cleanup_and_fail;
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if ((indexes_to_load & INDEX_GLOBE_COORDINATE) && !load_globe_coordinate_top_index(&(ctx->globe_coordinate_top_index), ctx->platform)) goto cleanup_and_fail;
#endif

    DEBUG_PRINT("db_init completed successfully.");
    return ctx;

cleanup_and_fail:
    db_end(ctx); 
    return NULL;
}

SearchCursor* search_begin(DatabaseContext* ctx, const SearchQuery* query) {
    if (ctx == NULL || query == NULL) return NULL;

    struct SearchCursor_t* cursor = (struct SearchCursor_t*)calloc(1, sizeof(struct SearchCursor_t));
    if (cursor == NULL) return NULL;

    cursor->ctx = ctx;
    cursor->query = *query; 
    cursor->end_of_results = false;
    cursor->current_row_index = 0;
    cursor->seen_qid_count = 0;

    switch (query->type) {
#if WIKI_PDA_ENABLE_OMNI_SEARCH
        case SEARCH_TYPE_OMNI:
            if (ctx->omni_top_index == NULL) goto fail;
            cursor->row_size = sizeof(OmniRow); 
            strncpy(cursor->target.omni_search_term, query->target.omni_search_term, OMNI_SEARCH_TERM_SIZE);
            cursor->target.omni_search_term[OMNI_SEARCH_TERM_SIZE - 1] = '\0';
            cursor->cached_term_length = strlen(cursor->target.omni_search_term);

            if (!omni_search(cursor->target.omni_search_term, ctx->omni_top_index, &cursor->next_read_offset, ctx->platform)) {
                cursor->end_of_results = true; 
            }
            break;
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
        case SEARCH_TYPE_GLOBE_COORDINATE:
            // if (ctx->globe_coordinate_top_index == NULL) goto fail;
            // cursor->row_size = sizeof(GlobeCoordinateRow); 
            // cursor->target.globe_coordinate_search_term = encode_globe_coordinates(
            //     query->target.globe_coordinate_search_term.lat, 
            //     query->target.globe_coordinate_search_term.lon
            // );
            //
            // if (!globe_coordinate_search(
            //         cursor->target.globe_coordinate_search_term, 
            //         ctx->globe_coordinate_top_index, 
            //         &cursor->next_read_offset, 
            //         ctx->platform)) {
            //     cursor->end_of_results = true;
            // }
            break;
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
        // case SEARCH_TYPE_ASTRONOMICAL:
        //     if (ctx->astronomical_top_index == NULL) goto fail;
        //     cursor->row_size = sizeof(AstronomicalRow); 
        //     cursor->target.astronomical_search_term = encode_astronomical_position(
        //         query->target.astronomical_search_term.dec, 
        //         query->target.astronomical_search_term.ra
        //     );
        //
        //     if (!astronomical_search(
        //             cursor->target.astronomical_search_term, 
        //             ctx->astronomical_top_index, 
        //             &cursor->next_read_offset, 
        //             ctx->platform)) {
        //         cursor->end_of_results = true;
        //     }
        break;
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
        case SEARCH_TYPE_TEMPORAL:
            // if (ctx->temporal_top_index == NULL) goto fail;
            // cursor->row_size = sizeof(TemporalRow); 
            // cursor->target.temporal_search_term = encode_time(query->target.temporal_iso_string);
            //
            // if (!temporal_search(
            //         cursor->target.temporal_search_term, 
            //         ctx->temporal_top_index, 
            //         &cursor->next_read_offset, 
            //         ctx->platform)) {
            //     cursor->end_of_results = true;
            // }
            break;
#endif

        default:
            goto fail;
    }

    return (SearchCursor*)cursor;

fail:
    free(cursor);
    return NULL;
}

bool search_next(SearchCursor* cursor, SearchResult* out_result) {
    if (cursor == NULL || cursor->end_of_results) return false;

    while (true) {
        if (cursor->current_row_index >= cursor->valid_rows_in_batch) {
            if (!cursor->ctx->platform.read_fn(cursor->next_read_offset, 
                                               cursor->raw_bytes, 
                                               512, 
                                               cursor->ctx->platform.user_data)) {
                cursor->end_of_results = true;
                return false;
            }
            cursor->next_read_offset += 512;
            cursor->current_row_index = 0;
            cursor->valid_rows_in_batch = 512 / cursor->row_size;
        }

        void* raw_row_ptr = cursor->raw_bytes + (cursor->current_row_index * cursor->row_size);
        cursor->current_row_index++;

        uint32_t qid = 0;
        uint32_t tags = 0;
        bool match_continues = false;

        switch (cursor->query.type) {
#if WIKI_PDA_ENABLE_OMNI_SEARCH
            case SEARCH_TYPE_OMNI: {
                OmniRow* row = (OmniRow*)raw_row_ptr;
                qid = row->qid;
                tags = row->tags;
                match_continues = (strncmp(row->term, cursor->target.omni_search_term, cursor->cached_term_length) == 0);
                
                // Copy the actual term found in the database to the title buffer
                snprintf(cursor->current_title_buffer, sizeof(cursor->current_title_buffer), 
                         "%.*s", (int)OMNI_SEARCH_TERM_SIZE, row->term);
                break;
            }
#endif
#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
            case SEARCH_TYPE_GLOBE_COORDINATE: {
                GlobeCoordinateRow* row = (GlobeCoordinateRow*)raw_row_ptr;
                qid = row->qid;
                tags = row->tags;
                match_continues = (row->term == cursor->target.globe_coordinate_search_term);
                
                // Format the int64 term as a string
                snprintf(cursor->current_title_buffer, sizeof(cursor->current_title_buffer), "%" PRId64, row->term);
                break;
            }
#endif
#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
            case SEARCH_TYPE_ASTRONOMICAL: {
                AstronomicalRow* row = (AstronomicalRow*)raw_row_ptr;
                qid = row->qid;
                tags = row->tags;
                match_continues = (row->term == cursor->target.astronomical_search_term);
                
                snprintf(cursor->current_title_buffer, sizeof(cursor->current_title_buffer), "%" PRId64, row->term);
                break;
            }
#endif
#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
            case SEARCH_TYPE_TEMPORAL: {
                TemporalRow* row = (TemporalRow*)raw_row_ptr;
                qid = row->qid;
                tags = row->tags;
                match_continues = (row->term == cursor->target.temporal_search_term);
                
                snprintf(cursor->current_title_buffer, sizeof(cursor->current_title_buffer), "%" PRId64, row->term);
                break;
            }
#endif
            default:
                cursor->end_of_results = true;
                return false;
        }

        if (!match_continues) {
            cursor->end_of_results = true;
            return false;
        }

        if (!_check_tags(tags, cursor->query.exact_tags, cursor->query.include_tags, cursor->query.exclude_tags)) {
            continue;
        }

        bool is_duplicate = false;
        uint16_t items_to_check = (cursor->seen_qid_count < MAX_DEDUPLICATION_CACHE) 
                                ? cursor->seen_qid_count : MAX_DEDUPLICATION_CACHE;
        for (uint16_t i = 0; i < items_to_check; i++) {
            if (cursor->seen_qids[i] == qid) {
                is_duplicate = true; 
                break;
            }
        }
        if (is_duplicate) continue;

        uint64_t relative_data_offset = 0;
        uint32_t data_length = 0;
        if (!get_relative_data_offset_and_length(qid, (uint16_t)cursor->query.article_type, 
                                                 &relative_data_offset, &data_length, cursor->ctx->platform)) {
            continue; 
        } 

        // Removed fetch_real_title(...) from here!

        cursor->seen_qids[(cursor->seen_qid_count++) % MAX_DEDUPLICATION_CACHE] = qid;

        out_result->qid = qid;
        out_result->tags = tags;
        out_result->article_type = cursor->query.article_type;
        out_result->title = cursor->current_title_buffer;
        out_result->data_length = data_length;
        out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);

        return true;
    }
}

bool search_end(SearchCursor* cursor) {
    DEBUG_PRINT("search_end called.");
    if (cursor == NULL) return false;
    free(cursor);
    DEBUG_PRINT("Cursor freed.");
    return true;
}

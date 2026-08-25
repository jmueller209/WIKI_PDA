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

typedef struct {
    uint32_t qid;
    uint32_t tags;
    float distance;
    float lat; // Also used for Dec (Declination) in Astro Search
    float lon; // Also used for RA (Right Ascension) in Astro Search
} SpatialMatch;

typedef struct {
    MortonRange ranges[MAX_MORTON_RANGES];
    uint8_t num_ranges;
    uint8_t current_range_index;

    SpatialMatch sorted_results[MAX_SORTED_RESULTS];
    uint16_t num_sorted_results;
    uint16_t current_sorted_index;

    CompareCtx compare_ctx;
} SpatialCursorState;

typedef struct {
    char search_term[OMNI_SEARCH_TERM_SIZE];
    size_t term_length;
} OmniCursorState;

typedef struct {
    int64_t date_code;
    bool search_forward;
} TemporalCursorState;

struct SearchCursor_t {
    DatabaseContext* ctx;
    SearchQuery query;
    bool end_of_results;

    uint64_t next_read_offset;

    uint8_t raw_bytes[512];
    uint8_t current_row_index;
    uint8_t valid_rows_in_batch;
    size_t row_size;

    uint32_t seen_qids[MAX_DEDUPLICATION_CACHE];
    uint32_t seen_qid_count;

    char article_title_buffer[256];
    char match_term_buffer[256];

    union {
        SpatialCursorState spatial;
        OmniCursorState omni;
        TemporalCursorState temporal;
    } state;
};

typedef enum {
    ROW_MATCH,
    ROW_SKIP,
    ROW_JUMP,
    ROW_END
} RowEvalResult;

bool _check_tags(uint32_t row_tags, SearchTagMask exact_tags, SearchTagMask include_tags, SearchTagMask exclude_tags) {
    if (exact_tags != 0 && row_tags != exact_tags) return false;
    if (include_tags != 0 && (row_tags & include_tags) != include_tags) return false;
    if (exclude_tags != 0 && (row_tags & exclude_tags) != 0) return false;
    return true;
}

void fetch_real_title(uint32_t qid, char* buffer, size_t max_len, DatabasePlatform platform) {
    strncpy(buffer, "Untitled", max_len - 1);
    buffer[max_len - 1] = '\0';
}

// TODO: Improve algorithm (pretty slow right now)
static void _insert_sorted_spatial_match(SpatialCursorState* spatial, uint32_t qid, uint32_t tags, float distance, float lat, float lon, uint16_t max_results) {
    if (max_results == 0 || max_results > MAX_SORTED_RESULTS) max_results = MAX_SORTED_RESULTS;

    for (int i = 0; i < spatial->num_sorted_results; i++) {
        if (spatial->sorted_results[i].qid == qid) {
            return;
        }
    }

    int insert_idx = -1;
    for (int i = 0; i < spatial->num_sorted_results; i++) {
        if (distance < spatial->sorted_results[i].distance) {
            insert_idx = i;
            break;
        }
    }

    if (insert_idx == -1) {
        if (spatial->num_sorted_results < max_results) {
            spatial->sorted_results[spatial->num_sorted_results].qid = qid;
            spatial->sorted_results[spatial->num_sorted_results].tags = tags;
            spatial->sorted_results[spatial->num_sorted_results].distance = distance;
            spatial->sorted_results[spatial->num_sorted_results].lat = lat;
            spatial->sorted_results[spatial->num_sorted_results].lon = lon;
            spatial->num_sorted_results++;
        }
    } else {
        int shift_end = spatial->num_sorted_results;
        if (shift_end >= max_results) shift_end = max_results - 1;

        for (int i = shift_end; i > insert_idx; i--) {
            spatial->sorted_results[i] = spatial->sorted_results[i - 1];
        }
        spatial->sorted_results[insert_idx].qid = qid;
        spatial->sorted_results[insert_idx].tags = tags;
        spatial->sorted_results[insert_idx].distance = distance;
        spatial->sorted_results[insert_idx].lat = lat;
        spatial->sorted_results[insert_idx].lon = lon;

        if (spatial->num_sorted_results < max_results) {
            spatial->num_sorted_results++;
        }
    }
}


#if WIKI_PDA_ENABLE_OMNI_SEARCH
static RowEvalResult _evaluate_omni_row(SearchCursor* cursor, void* raw_row_ptr, uint32_t* out_qid, uint32_t* out_tags) {
    OmniRow* row = (OmniRow*)raw_row_ptr;

    if (strncmp(row->term, cursor->state.omni.search_term, cursor->state.omni.term_length) == 0) {
        *out_qid = row->qid;
        *out_tags = row->tags;
        snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), 
                 "%.*s", (int)OMNI_SEARCH_TERM_SIZE, row->term);
        return ROW_MATCH;
    }

    DEBUG_PRINT("Omni-Search: Mismatch found. Terminating text search.");
    return ROW_END;
}
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
static RowEvalResult _evaluate_temporal_row(SearchCursor* cursor, void* raw_row_ptr, uint32_t* out_qid, uint32_t* out_tags) {
    bool forward = cursor->state.temporal.search_forward;

    uint64_t current_block_offset = cursor->next_read_offset + (forward ? -512 : 512);
    uint64_t offset_in_block = (uint64_t)((uint8_t*)raw_row_ptr - cursor->raw_bytes);
    uint64_t absolute_row_offset = current_block_offset + offset_in_block;

    uint64_t index_start = OFFSETS_TEMPORAL_SEARCH_LEVEL[0];
    uint64_t index_end = index_start + SIZES_TEMPORAL_SEARCH_LEVEL[0];

    if (forward) {
        if (absolute_row_offset >= index_end) return ROW_END;
        if (absolute_row_offset < index_start) return ROW_SKIP;
    } else {
        if (absolute_row_offset < index_start) return ROW_END;
        if (absolute_row_offset >= index_end) return ROW_SKIP; 
    }

    TemporalRow* row = (TemporalRow*)raw_row_ptr;

    if (row->qid == 0) return ROW_SKIP;

    int64_t target_date = cursor->state.temporal.date_code;

    if (forward) {
        if (row->term < target_date) return ROW_SKIP;
    } else {
        if (row->term > target_date) return ROW_SKIP;
    }

    *out_qid = row->qid;
    *out_tags = row->tags;

    TemporalDate date;
    if (temporal_decode(row->term, &date)) {
        snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), 
                 "%" PRId64 "-%02u-%02u", date.year, date.month, date.day);
    }
    return ROW_MATCH;
}
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
static RowEvalResult _evaluate_globe_row(SearchCursor* cursor, void* raw_row_ptr, uint32_t* out_qid, uint32_t* out_tags) {
    GlobeCoordinateRow* row = (GlobeCoordinateRow*)raw_row_ptr;
    SpatialCursorState* spatial = &cursor->state.spatial;

    if (spatial->current_range_index >= spatial->num_ranges) return ROW_END;
    MortonRange current_range = spatial->ranges[spatial->current_range_index];

    if (row->term > current_range.end_code) {
        spatial->current_range_index++;
        if (spatial->current_range_index >= spatial->num_ranges) {
            DEBUG_PRINT("Globe-Search: Reached last range.");
            return ROW_END;
        }

        uint64_t next_min = spatial->ranges[spatial->current_range_index].start_code;
        DEBUG_PRINT("Globe-Search: Jumping to next range (Start: %" PRIu64 ").", next_min);

        globe_coordinate_search(next_min, cursor->ctx->globe_coordinate_top_index, 
                            &cursor->next_read_offset, cursor->ctx->platform);
        return ROW_JUMP; 
    }

    if (row->term >= current_range.start_code) {
        float dist = spatial_code_is_in_radius(row->term, &spatial->compare_ctx);

        if (dist >= 0) {
            *out_qid = row->qid;
            *out_tags = row->tags;
            float row_lat, row_lon;
            spatial_decode(row->term, &row_lat, &row_lon, spatial->compare_ctx.spatialCtx);
            snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), "%.4f, %.4f", row_lat, row_lon);
            return ROW_MATCH;
        }
    }

    return ROW_SKIP; 
}
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
static RowEvalResult _evaluate_astronomical_row(SearchCursor* cursor, void* raw_row_ptr, uint32_t* out_qid, uint32_t* out_tags) {
    AstronomicalRow* row = (AstronomicalRow*)raw_row_ptr;
    SpatialCursorState* spatial = &cursor->state.spatial;

    if (spatial->current_range_index >= spatial->num_ranges) return ROW_END;
    MortonRange current_range = spatial->ranges[spatial->current_range_index];

    if (row->term > current_range.end_code) {
        spatial->current_range_index++;
        if (spatial->current_range_index >= spatial->num_ranges) {
            DEBUG_PRINT("Astro-Search: Reached last range.");
            return ROW_END;
        }

        uint64_t next_min = spatial->ranges[spatial->current_range_index].start_code;
        DEBUG_PRINT("Astro-Search: Jumping to next range (Start: %" PRIu64 ").", next_min);

        astronomical_search(next_min, cursor->ctx->astronomical_top_index, 
                            &cursor->next_read_offset, cursor->ctx->platform);
        return ROW_JUMP; 
    }

    if (row->term >= current_range.start_code) {
        float dist = spatial_code_is_in_radius(row->term, &spatial->compare_ctx);

        if (dist >= 0) {
            *out_qid = row->qid;
            *out_tags = row->tags;
            float row_dec, row_ra;
            spatial_decode(row->term, &row_dec, &row_ra, spatial->compare_ctx.spatialCtx);
            snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), "%.4f, %.4f", row_dec, row_ra);
            return ROW_MATCH;
        }
    }

    return ROW_SKIP; 
}
#endif

DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform) {
    DEBUG_PRINT("db_init called.");
    if (platform.read_fn == NULL) return NULL;
    DatabaseContext* ctx = (DatabaseContext*)calloc(1, sizeof(struct DatabaseContext_t));
    if (ctx == NULL) return NULL;
    ctx->platform = platform;

    if (!load_zstd_dictionary(&(ctx->zstd_dict), &(ctx->zstd_dict_length), ctx->platform)) {
        db_end(ctx); return NULL;
    }

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if ((indexes_to_load & INDEX_GLOBE_COORDINATE) && !load_globe_coordinate_top_index(&(ctx->globe_coordinate_top_index), ctx->platform)) {
        db_end(ctx); return NULL;
    }
#endif
#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    if ((indexes_to_load & INDEX_TEMPORAL) && !load_temporal_top_index(&(ctx->temporal_top_index), ctx->platform)) {
        db_end(ctx); return NULL;
    }
#endif
#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if ((indexes_to_load & INDEX_ASTRONOMICAL) && !load_astronomical_top_index(&(ctx->astronomical_top_index), ctx->platform)) {
        db_end(ctx); return NULL;
    }
#endif
#if WIKI_PDA_ENABLE_OMNI_SEARCH
    if ((indexes_to_load & INDEX_OMNI) && !load_omni_top_index(&(ctx->omni_top_index), ctx->platform)) {
        db_end(ctx); return NULL;
    }
#endif
    return ctx;
}

bool db_end(DatabaseContext* ctx) {
    if (ctx == NULL) return false;
    if (ctx->zstd_dict != NULL) free_zstd_dictionary(ctx->zstd_dict);
#if WIKI_PDA_ENABLE_OMNI_SEARCH
    if (ctx->omni_top_index != NULL) free_omni_top_index(ctx->omni_top_index); 
#endif
#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    if (ctx->temporal_top_index != NULL) free_temporal_top_index(ctx->temporal_top_index); 
#endif
#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if (ctx->globe_coordinate_top_index != NULL) free_globe_coordinate_top_index(ctx->globe_coordinate_top_index);
#endif
#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if (ctx->astronomical_top_index != NULL) free_astronomical_top_index(ctx->astronomical_top_index);
#endif
    free(ctx);
    return true;
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
        case SEARCH_TYPE_OMNI: {
            if (ctx->omni_top_index == NULL){
                DEBUG_PRINT("ERROR: omni_top_index is NULL!");
                goto fail;
            }
            cursor->row_size = sizeof(OmniRow); 
            strncpy(cursor->state.omni.search_term, query->target.omni.text, OMNI_SEARCH_TERM_SIZE);
            cursor->state.omni.search_term[OMNI_SEARCH_TERM_SIZE - 1] = '\0';
            cursor->state.omni.term_length = strlen(cursor->state.omni.search_term);
            DEBUG_PRINT("Omni-Search: Starting search for '%s'", cursor->state.omni.search_term);
            if (!omni_search(cursor->state.omni.search_term, ctx->omni_top_index, &cursor->next_read_offset, ctx->platform)) {
                cursor->end_of_results = true;
            }
            break;
        }
#endif



#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
        case SEARCH_TYPE_TEMPORAL: {
            if (ctx->temporal_top_index == NULL){
                DEBUG_PRINT("ERROR: temporal_top_index is NULL!");
                goto fail;
            }
            cursor->row_size = sizeof(TemporalRow);
            cursor->state.temporal.date_code = query->target.temporal.date_code;
            cursor->state.temporal.search_forward = query->target.temporal.search_forward;
            DEBUG_PRINT("Temporal-Search: Starting search for %" PRId64, cursor->state.temporal.date_code);
            if (!temporal_search(cursor->state.temporal.date_code, ctx->temporal_top_index, &cursor->next_read_offset, ctx->platform)){
                if (!cursor->state.temporal.search_forward) {
                    uint64_t index_start = OFFSETS_TEMPORAL_SEARCH_LEVEL[0];
                    uint64_t index_size  = SIZES_TEMPORAL_SEARCH_LEVEL[0];
                    cursor->next_read_offset = index_start + ((index_size - 1) / 512) * 512;
                    DEBUG_PRINT("Temporal-Search: Target out of bounds. Warping backward search to last block");
                } else {
                    cursor->end_of_results = true;
                }
            }
            DEBUG_PRINT("Offset: %" PRIu64 ").", cursor->next_read_offset);
            break;
        }
#endif

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
        case SEARCH_TYPE_GLOBE_COORDINATE: {
            if (ctx->globe_coordinate_top_index == NULL) {
                DEBUG_PRINT("ERROR: globe_coordinate_top_index is NULL!");
                goto fail;
            }
            float lat = query->target.globe.lat;
            float lon = query->target.globe.lon;
            float search_radius_km = query->target.globe.search_radius_km;
            SpatialzCtx spatial_ctx = spatial_create_earth_ctx();
            cursor->row_size = sizeof(GlobeCoordinateRow);

            int num_ranges_found = 0;
            if (!spatial_get_radius_ranges(lat, lon, search_radius_km,
                                           cursor->state.spatial.ranges,
                                           &num_ranges_found,
                                           MAX_MORTON_RANGES, &spatial_ctx)) {
                DEBUG_PRINT("ERROR: spatial_get_radius_ranges failed!");
                goto fail;
            }
            cursor->state.spatial.num_ranges = (uint8_t)num_ranges_found;
            cursor->state.spatial.current_range_index = 0;
            cursor->state.spatial.current_sorted_index = 0;
            cursor->state.spatial.num_sorted_results = 0;

            DEBUG_PRINT("Globe-Search: Found ranges: %d", num_ranges_found);
            if (num_ranges_found > 0) {
                DEBUG_PRINT("Globe-Search: First range -> Start: %" PRIu64 ", End: %" PRIu64,
                            cursor->state.spatial.ranges[0].start_code,
                            cursor->state.spatial.ranges[0].end_code);
            }

            if (cursor->state.spatial.num_ranges == 0) {
                DEBUG_PRINT("Globe-Search: No ranges found within radius.");
                cursor->end_of_results = true;
                break;
            }

            cursor->state.spatial.compare_ctx = spatial_create_compare_ctx(
                lat, lon, search_radius_km, spatial_ctx
            );

            if (query->target.globe.sort_by_distance) {
                DEBUG_PRINT("Globe-Search: Top-K mode active. Scanning ranges for sorted results...");
                uint16_t max_results = query->target.globe.max_results > 0 ? query->target.globe.max_results : MAX_SORTED_RESULTS;

                for (int r = 0; r < cursor->state.spatial.num_ranges; r++) {
                    MortonRange range = cursor->state.spatial.ranges[r];
                    uint64_t read_offset = 0;
                    if (!globe_coordinate_search(range.start_code, ctx->globe_coordinate_top_index, &read_offset, ctx->platform)) {
                        continue;
                    }

                    GlobeCoordinateRow row_batch[32];
                    while (true) {
                        if (!ctx->platform.read_fn(read_offset, (uint8_t*)row_batch, sizeof(row_batch), ctx->platform.user_data)) {
                            break;
                        }
                        size_t rows_in_batch = sizeof(row_batch) / sizeof(GlobeCoordinateRow);
                        bool out_of_range = false;

                        for (size_t i = 0; i < rows_in_batch; i++) {
                            GlobeCoordinateRow row = row_batch[i];
                            if (row.term > range.end_code) {
                                out_of_range = true;
                                break;
                            }
                            if (row.term >= range.start_code) {
                                float dist = spatial_code_is_in_radius(row.term, &cursor->state.spatial.compare_ctx);
                                if (dist >= 0.0) {
                                    float row_lat, row_lon;
                                    spatial_decode(row.term, &row_lat, &row_lon, cursor->state.spatial.compare_ctx.spatialCtx);
                                    _insert_sorted_spatial_match(&cursor->state.spatial, row.qid, row.tags, (float)dist, row_lat, row_lon, max_results);
                                }
                            }
                        }
                        if (out_of_range) break;
                        read_offset += sizeof(row_batch);
                    }
                }
                DEBUG_PRINT("Globe-Search: Top-K populated %d results.", cursor->state.spatial.num_sorted_results);
                if (cursor->state.spatial.num_sorted_results == 0) {
                    cursor->end_of_results = true;
                }
            } else {
                DEBUG_PRINT("Globe-Search: Stream mode active. Invoking globe_coordinate_search...");
                uint64_t target_code = cursor->state.spatial.ranges[0].start_code;
                bool search_success = globe_coordinate_search(
                    target_code, 
                    ctx->globe_coordinate_top_index, 
                    &cursor->next_read_offset, 
                    ctx->platform
                );

                if (!search_success) {
                    DEBUG_PRINT("Globe-Search: globe_coordinate_search returned false (no match in index).");
                    cursor->end_of_results = true;
                } else {
                    DEBUG_PRINT("Globe-Search: Start offset successfully determined: %" PRIu64, cursor->next_read_offset);
                }
            }
            break;
        }
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
        case SEARCH_TYPE_ASTRONOMICAL: {
            if (ctx->astronomical_top_index == NULL) {
                DEBUG_PRINT("ERROR: astronomical_top_index is NULL!");
                goto fail;
            }
            float dec = query->target.astronomical.dec;
            float ra = query->target.astronomical.ra;
            float search_radius = query->target.astronomical.search_radius_degrees;
            SpatialzCtx spatial_ctx = spatial_create_celestial_ctx();
            cursor->row_size = sizeof(AstronomicalRow);

            int num_ranges_found = 0;
            if (!spatial_get_radius_ranges(dec, ra, search_radius,
                                           cursor->state.spatial.ranges,
                                           &num_ranges_found,
                                           MAX_MORTON_RANGES, &spatial_ctx)) {
                DEBUG_PRINT("ERROR: spatial_get_radius_ranges failed!");
                goto fail;
            }
            cursor->state.spatial.num_ranges = (uint8_t)num_ranges_found;
            cursor->state.spatial.current_range_index = 0;
            cursor->state.spatial.current_sorted_index = 0;
            cursor->state.spatial.num_sorted_results = 0;

            DEBUG_PRINT("Astro-Search: Found ranges: %d", num_ranges_found);
            if (num_ranges_found > 0) {
                DEBUG_PRINT("Astro-Search: First range -> Start: %" PRIu64 ", End: %" PRIu64,
                            cursor->state.spatial.ranges[0].start_code,
                            cursor->state.spatial.ranges[0].end_code);
            }

            if (cursor->state.spatial.num_ranges == 0) {
                DEBUG_PRINT("Astro-Search: No ranges found within radius.");
                cursor->end_of_results = true;
                break;
            }

            cursor->state.spatial.compare_ctx = spatial_create_compare_ctx(
                dec, ra, search_radius, spatial_ctx
            );

            if (query->target.astronomical.sort_by_distance) {
                DEBUG_PRINT("Astro-Search: Top-K mode active. Scanning ranges for sorted results...");
                uint16_t max_results = query->target.astronomical.max_results > 0 ? query->target.astronomical.max_results : MAX_SORTED_RESULTS;

                for (int r = 0; r < cursor->state.spatial.num_ranges; r++) {
                    MortonRange range = cursor->state.spatial.ranges[r];
                    uint64_t read_offset = 0;
                    if (!astronomical_search(range.start_code, ctx->astronomical_top_index, &read_offset, ctx->platform)) {
                        continue;
                    }

                    AstronomicalRow row_batch[32];
                    while (true) {
                        if (!ctx->platform.read_fn(read_offset, (uint8_t*)row_batch, sizeof(row_batch), ctx->platform.user_data)) {
                            break;
                        }
                        size_t rows_in_batch = sizeof(row_batch) / sizeof(AstronomicalRow);
                        bool out_of_range = false;

                        for (size_t i = 0; i < rows_in_batch; i++) {
                            AstronomicalRow row = row_batch[i];
                            if (row.term > range.end_code) {
                                out_of_range = true;
                                break;
                            }
                            if (row.term >= range.start_code) {
                                float dist = spatial_code_is_in_radius(row.term, &cursor->state.spatial.compare_ctx); 
                                if (dist >= 0.0) {
                                    float row_dec, row_ra;
                                    spatial_decode(row.term, &row_dec, &row_ra, cursor->state.spatial.compare_ctx.spatialCtx);
                                    _insert_sorted_spatial_match(&cursor->state.spatial, row.qid, row.tags, (float)dist, row_dec, row_ra, max_results);
                                }
                            }
                        }
                        if (out_of_range) break;
                        read_offset += sizeof(row_batch);
                    }
                }
                DEBUG_PRINT("Astro-Search: Top-K populated %d results.", cursor->state.spatial.num_sorted_results);
                if (cursor->state.spatial.num_sorted_results == 0) {
                    cursor->end_of_results = true;
                }
            } else {
                DEBUG_PRINT("Astro-Search: Stream mode active. Invoking astronomical_search...");
                uint64_t target_code = cursor->state.spatial.ranges[0].start_code;
                bool search_success = astronomical_search(
                    target_code, 
                    ctx->astronomical_top_index, 
                    &cursor->next_read_offset, 
                    ctx->platform
                );

                if (!search_success) {
                    DEBUG_PRINT("Astro-Search: astronomical_search returned false (no match in index).");
                    cursor->end_of_results = true;
                }
            }
            break;
        }
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
    bool is_spatial_top_k = false;

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if (cursor->query.type == SEARCH_TYPE_GLOBE_COORDINATE && cursor->query.target.globe.sort_by_distance) {
        is_spatial_top_k = true;
    }
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if (cursor->query.type == SEARCH_TYPE_ASTRONOMICAL && cursor->query.target.astronomical.sort_by_distance) {
        is_spatial_top_k = true;
    }
#endif

    if (is_spatial_top_k) {
        while (cursor->state.spatial.current_sorted_index < cursor->state.spatial.num_sorted_results) {
            SpatialMatch match = cursor->state.spatial.sorted_results[cursor->state.spatial.current_sorted_index++];
            uint64_t relative_data_offset = 0;
            uint32_t data_length = 0;
            if (!get_relative_data_offset_and_length(match.qid, (uint16_t)cursor->query.article_type, 
                                                     &relative_data_offset, &data_length, cursor->ctx->platform)) {
                continue; 
            }

            out_result->qid = match.qid;
            out_result->tags = match.tags;
            out_result->article_type = cursor->query.article_type;
            out_result->data_length = data_length;
            out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);

            snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), "%.4f, %.4f", match.lat, match.lon);
            out_result->term = cursor->match_term_buffer;
            snprintf(cursor->article_title_buffer, sizeof(cursor->article_title_buffer), "Untitled");
            out_result->title = cursor->article_title_buffer;
            return true;        }
        cursor->end_of_results = true;
        return false;
    }

    bool is_reverse = (cursor->query.type == SEARCH_TYPE_TEMPORAL && !cursor->query.target.temporal.search_forward);

    while (true) {
        if (cursor->current_row_index >= cursor->valid_rows_in_batch) {

            if (!cursor->ctx->platform.read_fn(cursor->next_read_offset,
                                               cursor->raw_bytes, 512,
                                               cursor->ctx->platform.user_data)) {
                cursor->end_of_results = true;
                return false;
            }

            cursor->valid_rows_in_batch = 512 / cursor->row_size;

            if (is_reverse) {
                cursor->next_read_offset -= 512;
                cursor->current_row_index = cursor->valid_rows_in_batch - 1;
            } else {
                cursor->next_read_offset += 512;
                cursor->current_row_index = 0;
            }
        }

        void* raw_row_ptr = cursor->raw_bytes + (cursor->current_row_index * cursor->row_size);

        if (is_reverse) {
            cursor->current_row_index--; 
        } else {
            cursor->current_row_index++;
        }

        uint32_t qid = 0;
        uint32_t tags = 0;
        RowEvalResult eval_result = ROW_SKIP;
        switch (cursor->query.type) {

#if WIKI_PDA_ENABLE_OMNI_SEARCH
            case SEARCH_TYPE_OMNI:
                eval_result = _evaluate_omni_row(cursor, raw_row_ptr, &qid, &tags);
                break;
#endif
#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
            case SEARCH_TYPE_TEMPORAL:
                eval_result = _evaluate_temporal_row(cursor, raw_row_ptr, &qid, &tags);
                break;
#endif
#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
            case SEARCH_TYPE_GLOBE_COORDINATE:
                eval_result = _evaluate_globe_row(cursor, raw_row_ptr, &qid, &tags);
                break;
#endif
#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
            case SEARCH_TYPE_ASTRONOMICAL:
                eval_result = _evaluate_astronomical_row(cursor, raw_row_ptr, &qid, &tags);
                break;
#endif
            default:
                eval_result = ROW_END;
        }

        if (eval_result == ROW_END) {
            cursor->end_of_results = true;
            return false;
        }
        if (eval_result == ROW_SKIP) {
            continue;
        }
        if (eval_result == ROW_JUMP) {
            cursor->current_row_index = cursor->valid_rows_in_batch;
            continue;
        }

        if (!_check_tags(tags, cursor->query.exact_tags, cursor->query.include_tags, cursor->query.exclude_tags)) continue;

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

        cursor->seen_qids[(cursor->seen_qid_count++) % MAX_DEDUPLICATION_CACHE] = qid;

        out_result->qid = qid;
        out_result->tags = tags;
        out_result->article_type = cursor->query.article_type;
        out_result->data_length = data_length;
        out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);
        out_result->term = cursor->match_term_buffer;
        snprintf(cursor->article_title_buffer, sizeof(cursor->article_title_buffer), "Untitled");
        out_result->title = cursor->article_title_buffer;

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

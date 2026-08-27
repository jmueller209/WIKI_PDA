#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#include "../core/core.h"
#include "../common/common.h"

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

DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform) {
    DEBUG_PRINT("db_init called.");

    if (platform.read_fn == NULL) return NULL;

    DatabaseContext* ctx = (DatabaseContext*)calloc(1, sizeof(struct DatabaseContext_t));
    if (ctx == NULL) return NULL;

    ctx->platform = platform;

    if (!load_and_verify_header(ctx)) {
        free(ctx);
        return NULL;
    }


    if (!load_zstd_dictionary(&(ctx->zstd_dict), &(ctx->zstd_dict_length), ctx->platform)) {
        db_end(ctx);
        return NULL;
    }

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
    if ((indexes_to_load & INDEX_GLOBE_COORDINATE) && ctx->header.globe_search.is_enabled) {
        if (!load_globe_coordinate_top_index(&(ctx->globe_coordinate_top_index), ctx->platform)) {
            db_end(ctx); return NULL;
        }
    }
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
    if ((indexes_to_load & INDEX_TEMPORAL) && ctx->header.temporal_search.is_enabled) {
        if (!load_temporal_top_index(&(ctx->temporal_top_index), ctx->platform)) {
            db_end(ctx); return NULL;
        }
    }
#endif

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
    if ((indexes_to_load & INDEX_ASTRONOMICAL) && ctx->header.astro_search.is_enabled) {
        if (!load_astronomical_top_index(&(ctx->astronomical_top_index), ctx->platform)) {
            db_end(ctx); return NULL;
        }
    }
#endif

#if WIKI_PDA_ENABLE_OMNI_SEARCH
    if ((indexes_to_load & INDEX_OMNI) && ctx->header.omni_search.is_enabled) {
        if (!load_omni_top_index(&(ctx->omni_top_index), ctx->platform)) {
            db_end(ctx); return NULL;
        }
    }
#endif

    DEBUG_PRINT("INIT SUCCESS: Database ready.");
    return ctx;
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
                                    insert_sorted_spatial_match(&cursor->state.spatial, row.qid, row.tags, (float)dist, row_lat, row_lon, max_results);
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
                                    insert_sorted_spatial_match(&cursor->state.spatial, row.qid, row.tags, (float)dist, row_dec, row_ra, max_results);
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
        case SEARCH_TYPE_QID: {
            cursor->row_size = sizeof(QIDHashMapRow);
            cursor->state.id.id = query->target.qid.id;
            cursor->state.id.search_forward = query->target.qid.search_forward;
            DEBUG_PRINT("QID-Search: Starting search for %" PRId64, cursor->state.id.id);
            break;
        }

        case SEARCH_TYPE_PID: {
            cursor->row_size = sizeof(PIDHashMapRow);
            cursor->state.id.id = query->target.pid.id;
            cursor->state.id.search_forward = query->target.pid.search_forward;
            DEBUG_PRINT("PID-Search: Starting search for %" PRId64, cursor->state.id.id);
            break;
        }

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
    if (cursor->query.type == SEARCH_TYPE_QID || cursor->query.type == SEARCH_TYPE_PID) {
        return search_next_id(cursor, out_result);
    }else{
        return search_next_in_index(cursor, out_result);
    }
}

bool search_end(SearchCursor* cursor) {
    DEBUG_PRINT("search_end called.");
    if (cursor == NULL) return false;
    free(cursor);
    DEBUG_PRINT("Cursor freed.");
    return true;
}

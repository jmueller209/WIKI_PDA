#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#include "../common/common.h"

#include "../indexes/qid_search.h"
#include "../indexes/pid_search.h"
#include "../indexes/omni_search.h"
#include "../indexes/astronomical_search.h"
#include "../indexes/temporal_search.h"
#include "../indexes/globe_coordinate_search.h"
#include "../../lib/tempus/include/tempus.h"
#include "../../lib/spatial_z/include/spatial_z.h"

bool static _check_tags(uint32_t row_tags, SearchTagMask exact_tags, SearchTagMask include_tags, SearchTagMask exclude_tags) {
    if (exact_tags != 0 && row_tags != exact_tags) return false;
    if (include_tags != 0 && (row_tags & include_tags) != include_tags) return false;
    if (exclude_tags != 0 && (row_tags & exclude_tags) != 0) return false;
    return true;
}

// TODO: Improve algorithm (pretty slow right now)
void insert_sorted_spatial_match(SpatialCursorState* spatial, uint32_t qid, uint32_t tags, float distance, float lat, float lon, uint16_t max_results) {
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

    uint64_t index_start = cursor->ctx->header.temporal_search.level_offsets[0];
    uint64_t index_end = index_start + cursor->ctx->header.temporal_search.level_sizes[0];

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
                            &cursor->next_read_offset, cursor->ctx);
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
                            &cursor->next_read_offset, cursor->ctx);
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



bool search_next_id(SearchCursor* cursor, SearchResult* out_result) {
    switch (cursor->query.type) {
        case SEARCH_TYPE_QID: {
            int dir = cursor->state.id.search_forward ? 1 : -1;
            uint32_t max_valid_qid = cursor->ctx->header.size_qid_hashmap / sizeof(QIDHashMapRow);
            // uint32_t max_valid_qid = SIZES_QID_HASHMAP / sizeof(QIDHashMapRow);

            if (cursor->state.id.id == cursor->query.target.qid.id && !cursor->query.target.qid.first_result_must_match) {

                if (!cursor->state.id.search_forward && cursor->state.id.id > max_valid_qid) {
                    cursor->state.id.id = max_valid_qid;
                }
                else if (cursor->state.id.search_forward && cursor->state.id.id == 0) {
                    cursor->state.id.id = 1;
                }
            }

            while (!cursor->end_of_results) {
                uint32_t current_id = cursor->state.id.id;

                if (current_id == 0 || current_id > max_valid_qid) {
                    cursor->end_of_results = true;
                    return false;
                }

                cursor->state.id.id += dir;

                uint64_t relative_data_offset = 0;
                uint32_t title_offset = 0;
                if (get_article_index_data(current_id, (uint16_t)cursor->query.article_type, &relative_data_offset, &out_result->data_length, &title_offset, cursor->ctx)) {
                    // out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);
                    out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? cursor->ctx->header.offset_metadata : cursor->ctx->header.offset_content);
                    out_result->article_type = cursor->query.article_type;
                    out_result->id = current_id;
                    out_result->tags = 0;
                    out_result->term = "";

                    get_article_title(title_offset, cursor->article_title_buffer, sizeof(cursor->article_title_buffer), cursor->ctx);
                    out_result->title = cursor->article_title_buffer;

                    return true;
                } else {
                    if (current_id == cursor->query.target.qid.id && cursor->query.target.qid.first_result_must_match) {
                        cursor->end_of_results = true;
                        return false;
                    }
                }
            }
            return false;
        }
        case SEARCH_TYPE_PID: {
            int dir = cursor->state.id.search_forward ? 1 : -1;
            // uint32_t max_valid_pid = SIZES_PID_HASHMAP / sizeof(PIDHashMapRow);
            uint32_t max_valid_pid = cursor->ctx->header.size_pid_hashmap / sizeof(PIDHashMapRow);

            if (cursor->state.id.id == cursor->query.target.pid.id && !cursor->query.target.qid.first_result_must_match) {
                if (!cursor->state.id.search_forward && cursor->state.id.id > max_valid_pid) {
                    cursor->state.id.id = max_valid_pid;
                }
                else if (cursor->state.id.search_forward && cursor->state.id.id == 0) {
                    cursor->state.id.id = 1;
                }
            }

            while (!cursor->end_of_results) {
                uint32_t current_id = cursor->state.id.id;

                if (current_id == 0 || current_id > max_valid_pid) {
                    cursor->end_of_results = true;
                    return false;
                }

                cursor->state.id.id += dir;

                uint32_t title_offset = 0;
                uint32_t desc_offset = 0;

                if (get_property_index_data(current_id, (uint16_t)cursor->query.article_type, &title_offset, &desc_offset, cursor->ctx)) {

                    out_result->data_offset = 0; 
                    out_result->data_length = 0;
                    out_result->article_type = cursor->query.article_type;
                    out_result->id = current_id;
                    out_result->tags = 0;

                    get_property_title(title_offset, cursor->article_title_buffer, sizeof(cursor->article_title_buffer), cursor->ctx);
                    out_result->title = cursor->article_title_buffer;

                    get_property_desc(desc_offset, cursor->match_term_buffer, sizeof(cursor->match_term_buffer), cursor->ctx);
                    out_result->term = cursor->match_term_buffer;

                    return true;
                } else {
                    if (current_id == cursor->query.target.pid.id && cursor->query.target.pid.first_result_must_match) {
                        cursor->end_of_results = true;
                        return false;
                    }
                }
            }
            return false;
        }
        default: {
            return false;
        }
    }
}

bool search_next_in_index(SearchCursor* cursor, SearchResult* out_result) {
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
            uint32_t title_offset = 0;
            if (!get_article_index_data(match.qid, (uint16_t)cursor->query.article_type, 
                                        &relative_data_offset, &data_length, &title_offset, cursor->ctx)) {
                continue;
            }
            out_result->id = match.qid;
            out_result->tags = match.tags;
            out_result->article_type = cursor->query.article_type;
            out_result->data_length = data_length;
            // out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);
            out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? cursor->ctx->header.offset_metadata : cursor->ctx->header.offset_content);

            snprintf(cursor->match_term_buffer, sizeof(cursor->match_term_buffer), "%.4f, %.4f", match.lat, match.lon);
            out_result->term = cursor->match_term_buffer;

            get_article_title(title_offset, cursor->article_title_buffer, sizeof(cursor->article_title_buffer), cursor->ctx);
            out_result->title = cursor->article_title_buffer;

            return true;
        }
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
        uint32_t title_offset = 0;
        if (!get_article_index_data(qid, (uint16_t)cursor->query.article_type,
                                    &relative_data_offset, &data_length, &title_offset, cursor->ctx)) {
            continue;
        }

        cursor->seen_qids[(cursor->seen_qid_count++) % MAX_DEDUPLICATION_CACHE] = qid;

        out_result->id = qid;
        out_result->tags = tags;
        out_result->article_type = cursor->query.article_type;
        out_result->data_length = data_length;
        // out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? OFFSETS_METADATA : OFFSETS_CONTENT);
        out_result->data_offset = relative_data_offset + (cursor->query.article_type == 0 ? cursor->ctx->header.offset_metadata : cursor->ctx->header.offset_content);
        out_result->term = cursor->match_term_buffer;

        get_article_title(title_offset, cursor->article_title_buffer, sizeof(cursor->article_title_buffer), cursor->ctx);
        out_result->title = cursor->article_title_buffer;

        return true;
    }
}


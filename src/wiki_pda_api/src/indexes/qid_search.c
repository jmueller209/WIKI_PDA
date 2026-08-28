// qid_search.c
#include "qid_search.h"
#include <stddef.h>
#include <stdlib.h>
#include "../common/common.h"

static bool _get_qid_hash_map_row(uint32_t qid, QIDHashMapRow* out_hash_map_row, DatabaseContext* ctx) {
    if (qid == 0 || out_hash_map_row == NULL || ctx == NULL) {
        return false;
    }
    uint32_t max_valid_qid = ctx->header.size_qid_hashmap / sizeof(QIDHashMapRow);

    if (qid > max_valid_qid) {
        DEBUG_PRINT("QID %u is out of bounds (Max: %u)", qid, max_valid_qid);
        return false;
    }

    uint64_t byte_offset = ((uint64_t)(qid - 1) * sizeof(QIDHashMapRow)) + ctx->header.offset_qid_hashmap;
    if (!ctx->platform.read_fn(byte_offset, (uint8_t*)out_hash_map_row, sizeof(QIDHashMapRow), ctx->platform.user_data)) {
        return false;
    }
    return true;
}

static bool _get_qid_index_rows(const QIDHashMapRow* row, QIDIndexRow* out_index_rows, DatabaseContext* ctx) {
    if (row == NULL || out_index_rows == NULL || ctx == NULL) {
        return false;
    }

    if (row->entry_count == 0) {
        return true;
    }

    uint64_t byte_offset = ((uint64_t)row->start_index * sizeof(QIDIndexRow)) + ctx->header.offset_qid_index;
    uint32_t total_bytes = (uint32_t)row->entry_count * sizeof(QIDIndexRow);

    if (!ctx->platform.read_fn(byte_offset, (uint8_t*)out_index_rows, total_bytes, ctx->platform.user_data)) {
        return false;
    }

    return true;
}

static bool _get_all_index_rows_for_qid(uint32_t qid, QIDIndexRow** out_index_rows, uint16_t* out_num_rows, DatabaseContext* ctx) {
    if (qid == 0 || out_index_rows == NULL || out_num_rows == NULL || ctx == NULL) {
        return false;
    }

    QIDHashMapRow hash_map_row;
    if (!_get_qid_hash_map_row(qid, &hash_map_row, ctx)) {
        return false;
    }

    if (hash_map_row.entry_count == 0) {
        return false;
    }

    QIDIndexRow* allocated_rows = (QIDIndexRow*)malloc(hash_map_row.entry_count * sizeof(QIDIndexRow));
    if (allocated_rows == NULL) {
        return false; 
    }

    if (!_get_qid_index_rows(&hash_map_row, allocated_rows, ctx)) {
        free(allocated_rows);
        return false;
    }

    *out_index_rows = allocated_rows;
    *out_num_rows = hash_map_row.entry_count;

    return true;
}

bool get_article_index_data(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length, uint32_t* out_title_offset, DatabaseContext* ctx) {
    if (qid == 0 || out_data_offset == NULL || out_data_length == NULL || out_title_offset == NULL || ctx == NULL) {
        return false;
    }

    QIDIndexRow* index_rows = NULL;
    uint16_t num_rows = 0;
    if (!_get_all_index_rows_for_qid(qid, &index_rows, &num_rows, ctx)) {
        return false;
    }

    bool found = false;
    for (uint16_t i = 0; i < num_rows; i++) {
        if (index_rows[i].project_id == project_id) {
            *out_data_offset = index_rows[i].offset;
            *out_data_length = index_rows[i].length;
            *out_title_offset = index_rows[i].title_offset;
            found = true;
            break;
        }
    }

    free(index_rows);
    return found;
}

bool get_article_title(uint32_t title_offset, char* out_title, size_t max_length, DatabaseContext* ctx) {
    if (out_title == NULL || max_length == 0 || ctx == NULL) {
        return false;
    }
    if (title_offset == 0) {
        out_title[0] = '\0';
        return true;
    }

    uint64_t absolute_offset = ctx->header.offset_titles + title_offset;
    if (!ctx->platform.read_fn(absolute_offset, (uint8_t*)out_title, max_length - 1, ctx->platform.user_data)) {
        out_title[0] = '\0';
        return false;
    }

    out_title[max_length - 1] = '\0'; 

    return true;
}

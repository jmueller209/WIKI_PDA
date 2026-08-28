// pid_search.c
#include "pid_search.h"
#include <stddef.h>
#include <stdlib.h>
#include "../common/common.h"

static bool _get_pid_hash_map_row(uint32_t pid, PIDHashMapRow* out_hash_map_row, DatabaseContext* ctx) {
    if (pid == 0 || out_hash_map_row == NULL || ctx == NULL) return false;

    uint32_t max_valid_pid = ctx->header.size_pid_hashmap / sizeof(PIDHashMapRow);
    if (pid > max_valid_pid) return false;

    uint64_t byte_offset = ((uint64_t)(pid - 1) * sizeof(PIDHashMapRow)) + ctx->header.offset_pid_hashmap;

    if (!ctx->platform.read_fn(byte_offset, (uint8_t*)out_hash_map_row, sizeof(PIDHashMapRow), ctx->platform.user_data)) {
        return false;
    }
    return true;
}

static bool _get_pid_index_rows(const PIDHashMapRow* row, PIDIndexRow* out_index_rows, DatabaseContext* ctx) {
    if (row == NULL || out_index_rows == NULL || ctx == NULL) {
        return false;
    }

    if (row->entry_count == 0) {
        return true;
    }

    uint64_t byte_offset = ((uint64_t)row->start_index * sizeof(PIDIndexRow)) + ctx->header.offset_pid_index;
    uint32_t total_bytes = (uint32_t)row->entry_count * sizeof(PIDIndexRow);

    if (!ctx->platform.read_fn(byte_offset, (uint8_t*)out_index_rows, total_bytes, ctx->platform.user_data)) {
        return false;
    }

    return true;
}

static bool _get_all_index_rows_for_pid(uint32_t pid, PIDIndexRow** out_index_rows, uint16_t* out_num_rows, DatabaseContext* ctx) {
    if (pid == 0 || out_index_rows == NULL || out_num_rows == NULL || ctx == NULL) {
        return false;
    }

    PIDHashMapRow hash_map_row;
    if (!_get_pid_hash_map_row(pid, &hash_map_row, ctx)) {
        return false;
    }

    if (hash_map_row.entry_count == 0) {
        return false;
    }

    PIDIndexRow* allocated_rows = (PIDIndexRow*)malloc(hash_map_row.entry_count * sizeof(PIDIndexRow));
    if (allocated_rows == NULL) {
        return false; 
    }

    if (!_get_pid_index_rows(&hash_map_row, allocated_rows, ctx)) {
        free(allocated_rows);
        return false;
    }

    *out_index_rows = allocated_rows;
    *out_num_rows = hash_map_row.entry_count;

    return true;
}

bool get_property_index_data(uint32_t pid, uint16_t lang_id, uint32_t* out_title_offset, uint32_t* out_desc_offset, DatabaseContext* ctx) {
    if (pid == 0 || out_title_offset == NULL || out_desc_offset == NULL || ctx == NULL) return false;

    PIDIndexRow* index_rows = NULL;
    uint16_t num_rows = 0;

    if (!_get_all_index_rows_for_pid(pid, &index_rows, &num_rows, ctx)) {
        return false;
    }

    bool found = false;
    for (uint16_t i = 0; i < num_rows; i++) {
        if (index_rows[i].project_id == lang_id) {
            *out_title_offset = index_rows[i].title_offset;
            *out_desc_offset  = index_rows[i].desc_offset;
            found = true;
            break;
        }
    }

    free(index_rows);
    return found;
}

bool get_property_title(uint32_t title_offset, char* out_title, size_t max_length, DatabaseContext* ctx) {
    if (out_title == NULL || max_length == 0 || ctx == NULL) {
        return false;
    }
    if (title_offset == 0) {
        out_title[0] = '\0';
        return true;
    }

    uint64_t absolute_offset = ctx->header.offset_pid_strings + title_offset;
    if (!ctx->platform.read_fn(absolute_offset, (uint8_t*)out_title, max_length - 1, ctx->platform.user_data)) {
        out_title[0] = '\0';
        return false;
    }

    out_title[max_length - 1] = '\0'; 

    return true;
}

bool get_property_desc(uint32_t descr_offset, char* out_descr, size_t max_length, DatabaseContext* ctx) {
    if (out_descr == NULL || max_length == 0 || ctx == NULL) {
        return false;
    }
    if (descr_offset == 0) {
        out_descr[0] = '\0';
        return true;
    }

    uint64_t absolute_offset = ctx->header.offset_pid_strings + descr_offset;
    if (!ctx->platform.read_fn(absolute_offset, (uint8_t*)out_descr, max_length - 1, ctx->platform.user_data)) {
        out_descr[0] = '\0';
        return false;
    }

    out_descr[max_length - 1] = '\0'; 

    return true;
}

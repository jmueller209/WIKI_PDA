#include "qid_search.h"
#include <stddef.h>
#include <stdlib.h>

#include "../../include/database_platform.h"

bool get_qid_hash_map_row(uint32_t qid, HashMapRow* out_hash_map_row, DatabasePlatform platform) {
    if (qid == 0 || out_hash_map_row == NULL) {
        return false;
    }
    uint64_t byte_offset = ((uint64_t)(qid - 1) * sizeof(HashMapRow)) + OFFSETS_QID_HASHMAP;
    if (!platform.read_fn(byte_offset, (uint8_t*)out_hash_map_row, sizeof(HashMapRow), platform.user_data)) {
        return false;
    }
    return true;
}

bool get_qid_index_rows(const HashMapRow* row, IndexRow* out_index_rows, DatabasePlatform platform) {
    if (row == NULL || out_index_rows == NULL) {
        return false;
    }

    if (row->entry_count == 0) {
        return true;
    }

    uint64_t byte_offset = ((uint64_t)row->start_index * sizeof(IndexRow)) + OFFSETS_QID_INDEX;
    uint32_t total_bytes = (uint32_t)row->entry_count * sizeof(IndexRow);

    if (!platform.read_fn(byte_offset, (uint8_t*)out_index_rows, total_bytes, platform.user_data)) {
        return false;
    }

    return true;
}

bool get_all_index_rows_for_qid(uint32_t qid, IndexRow** out_index_rows, uint16_t* out_num_rows, DatabasePlatform platform) {
    if (qid == 0 || out_index_rows == NULL || out_num_rows == NULL) {
        return false;
    }

    HashMapRow hash_map_row;
    if (!get_qid_hash_map_row(qid, &hash_map_row, platform)) {
        return false;
    }

    if (hash_map_row.entry_count == 0) {
        return false;
    }

    IndexRow* allocated_rows = (IndexRow*)malloc(hash_map_row.entry_count * sizeof(IndexRow));
    if (allocated_rows == NULL) {
        return false; 
    }

    if (!get_qid_index_rows(&hash_map_row, allocated_rows, platform)) {
        free(allocated_rows);
        return false;
    }

    *out_index_rows = allocated_rows;
    *out_num_rows = hash_map_row.entry_count;

    return true;
}

bool get_relative_data_offset_and_length(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length, DatabasePlatform platform) {
    if (qid == 0 || out_data_offset == NULL || out_data_length == NULL) {
        return false;
    }

    IndexRow* index_rows = NULL;
    uint16_t num_rows = 0;
    if (!get_all_index_rows_for_qid(qid, &index_rows, &num_rows, platform)) {
        return false;
    }

    bool found = false;
    for (uint16_t i = 0; i < num_rows; i++) {
        if (index_rows[i].project_id == project_id) {
            *out_data_offset = index_rows[i].offset;
            *out_data_length = index_rows[i].length;
            found = true;
            break;
        }
    }

    free(index_rows);
    return found;

}



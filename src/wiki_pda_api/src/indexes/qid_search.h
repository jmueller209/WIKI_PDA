#ifndef QID_SEARCH_H
#define QID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

typedef struct __attribute__((packed)) {
	uint32_t start_index;
	uint16_t entry_count;
} QIDHashMapRow;

typedef struct __attribute__((packed)) {
    uint64_t offset;
    uint32_t length;
    uint16_t project_id;
} QIDIndexRow; 

bool get_qid_hash_map_row(uint32_t qid, QIDHashMapRow* out_hash_map_row, DatabasePlatform platform);

bool get_qid_index_rows(const QIDHashMapRow* row, QIDIndexRow* out_index_rows, DatabasePlatform platform);

bool get_all_index_rows_for_qid(uint32_t qid, QIDIndexRow** out_index_rows, uint16_t* out_num_rows, DatabasePlatform platform);

bool get_relative_data_offset_and_length(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length, DatabasePlatform platform);


#endif

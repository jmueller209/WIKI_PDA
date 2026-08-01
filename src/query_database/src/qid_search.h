#ifndef QID_SEARCH_H
#define QID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "database_constants.h"
#include "database_io.h"

typedef struct __attribute__((packed)) {
	uint32_t start_index;
	uint16_t entry_count;
} HashMapRow; 


typedef struct __attribute__((packed)) {
    uint64_t offset;
    uint32_t length;
    uint16_t project_id;
} IndexRow; 

bool get_qid_hash_map_row(uint32_t qid, HashMapRow* out_hash_map_row);

bool get_qid_index_rows(const HashMapRow* row, IndexRow* out_index_rows);

bool get_all_index_rows_for_qid(uint32_t qid, IndexRow** out_index_rows, uint16_t* out_num_rows);
    
bool get_data_offset_and_length(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length);

bool get_metadata_offset_and_length(uint32_t qid, uint64_t* out_metadata_offset, uint32_t* out_metadata_length);

#endif

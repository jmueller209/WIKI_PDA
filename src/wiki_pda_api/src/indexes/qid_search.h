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
    uint32_t title_offset;
} QIDIndexRow;

bool get_article_index_data(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length, uint32_t* out_title_offset, DatabasePlatform platform);

bool get_article_title(uint32_t title_offset, char* out_title, size_t max_length, DatabasePlatform platform);

#endif

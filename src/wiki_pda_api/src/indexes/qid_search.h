// qid_search.h
#ifndef QID_SEARCH_H
#define QID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../../include/wiki_pda_types.h"

#ifdef __cplusplus
extern "C" {
#endif

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

bool get_article_index_data(uint32_t qid, uint16_t project_id, uint64_t* out_data_offset, uint32_t* out_data_length, uint32_t* out_title_offset, DatabaseContext* ctx);

bool get_article_title(uint32_t title_offset, char* out_title, size_t max_length, DatabaseContext* ctx);

#ifdef __cplusplus
}
#endif

#endif // QID_SEARCH_H

// pid_search.h
#ifndef PID_SEARCH_H
#define PID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "../../include/wiki_pda_types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct __attribute__((packed)) {
    uint32_t start_index;
    uint16_t entry_count;
} PIDHashMapRow;

typedef struct __attribute__((packed)) {
    uint16_t project_id;
    uint32_t title_offset;
    uint32_t desc_offset;
} PIDIndexRow;

bool get_property_index_data(uint32_t pid, uint16_t lang_id, uint32_t* out_title_offset, uint32_t* out_desc_offset, DatabaseContext* ctx);

bool get_property_title(uint32_t title_offset, char* out_title, size_t max_length, DatabaseContext* ctx);

bool get_property_desc(uint32_t descr_offset, char* out_descr, size_t max_length, DatabaseContext* ctx);

#ifdef __cplusplus
}
#endif

#endif // PID_SEARCH_H

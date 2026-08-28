// temporal_search.h
#ifndef TEMPORAL_SEARCH_H
#define TEMPORAL_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../../include/wiki_pda_platforms.h"
#include "../../include/wiki_pda_options.h"
#include "../../include/wiki_pda_types.h"

#ifdef __cplusplus
extern "C" {
#endif

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} TemporalRow;

typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t target_row;
    uint8_t _padding[4];
} TemporalSparseRow;

bool load_temporal_top_index(TemporalSparseRow** out_top_level_index, DatabaseContext* ctx);

void free_temporal_top_index(TemporalSparseRow* top_level_index);

bool temporal_search(
    int64_t search_term,
    const TemporalSparseRow* top_level_ram_index,
    uint64_t* out_abs_pointer,
    DatabaseContext* ctx
);

#endif

#ifdef __cplusplus
}
#endif

#endif // TEMPORAL_SEARCH_H

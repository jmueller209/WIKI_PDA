#ifndef OMNI_SEARCH_H
#define OMNI_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../../include/wiki_pda_platforms.h"
#include "../../include/wiki_pda_options.h"
#include "../../include/wiki_pda_types.h"


#if WIKI_PDA_ENABLE_OMNI_SEARCH

#ifdef __cplusplus
extern "C" {
#endif

typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t qid;
    uint32_t tags;
} OmniRow;

typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t target_row;
    uint8_t _padding[4];
} OmniSparseRow;

bool load_omni_top_index(OmniSparseRow** out_top_level_index, DatabaseContext* ctx);
void free_omni_top_index(OmniSparseRow* top_level_index);

bool omni_search(
    const char* search_term,
    const OmniSparseRow* top_level_index,
    uint64_t* out_abs_pointer,
    DatabaseContext* ctx
);

#endif

#ifdef __cplusplus
}
#endif

#endif // OMNI_SEARCH_H

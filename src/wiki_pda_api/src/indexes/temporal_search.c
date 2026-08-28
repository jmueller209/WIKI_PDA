// temporal_search.c
#include "../../include/wiki_pda_options.h"
#include "../common/common.h"

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH

#include "temporal_search.h"
#include "generic_search.h"

bool load_temporal_top_index(TemporalSparseRow** out_top_level_index,DatabaseContext* ctx) {
    if (ctx == NULL) return false;

    return load_top_level_index_generic(
        (void**)out_top_level_index,
        ctx->header.temporal_search.top_level_rows,
        sizeof(TemporalSparseRow),
        ctx->header.temporal_search.level_offsets[ctx->header.temporal_search.num_sparse_levels],
        ctx->platform,
        "Temporal"
    );
}

void free_temporal_top_index(TemporalSparseRow* top_level_index) {
    free_top_level_index_generic((void*) top_level_index);
}

bool temporal_search(
    int64_t search_term,
    const TemporalSparseRow* top_level_ram_index,
    uint64_t* out_abs_pointer,
    DatabaseContext* ctx
) {
    if (ctx == NULL) return false;

    return generic_int64_search(
        search_term,
        (const void*)top_level_ram_index,
        ctx->header.temporal_search.top_level_rows,
        ctx->header.temporal_search.num_sparse_levels,
        ctx->header.temporal_search.chunk_size,
        ctx->header.temporal_search.level_offsets,
        ctx->header.temporal_search.level_sizes,
        sizeof(TemporalSparseRow),
        sizeof(TemporalRow),
        out_abs_pointer,
        ctx->platform
    );
}

#endif

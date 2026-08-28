#include "../../include/wiki_pda_options.h"

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH

#include "astronomical_search.h"
#include "generic_search.h"
#include "../common/common.h"

bool load_astronomical_top_index(AstronomicalSparseRow** out_top_level_index, DatabaseContext* ctx) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        ctx->header.astro_search.top_level_rows,
        sizeof(AstronomicalSparseRow),
        ctx->header.astro_search.level_offsets[ctx->header.astro_search.num_sparse_levels],
        ctx->platform,
        "Astronomical"
    );
}

void free_astronomical_top_index(AstronomicalSparseRow* top_level_index) {
    free_top_level_index_generic((void*) top_level_index);
}

bool astronomical_search(
    uint64_t search_term,
    const AstronomicalSparseRow* top_level_ram_index,
    uint64_t* out_abs_pointer,
    DatabaseContext* ctx
) {
    return generic_int64_search(
        search_term,
        (const void*)top_level_ram_index,
        ctx->header.astro_search.top_level_rows,
        ctx->header.astro_search.num_sparse_levels,
        ctx->header.astro_search.chunk_size,
        ctx->header.astro_search.level_offsets,
        ctx->header.astro_search.level_sizes,
        sizeof(AstronomicalSparseRow),
        sizeof(AstronomicalRow),
        out_abs_pointer,
        ctx->platform
    );
}

#endif

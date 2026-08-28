#include "../../include/wiki_pda_options.h"

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH

#include "globe_coordinate_search.h"
#include "generic_search.h"
#include "../common/common.h"

bool load_globe_coordinate_top_index(GlobeCoordinateSparseRow** out_top_level_index, DatabaseContext* ctx) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        ctx->header.globe_search.top_level_rows,
        sizeof(GlobeCoordinateSparseRow),
        ctx->header.globe_search.level_offsets[ctx->header.globe_search.num_sparse_levels],
        ctx->platform,
        "Globe Coordinate"
    );
}

void free_globe_coordinate_top_index(GlobeCoordinateSparseRow* top_level_index) {
    free_top_level_index_generic((void*) top_level_index);
}

bool globe_coordinate_search(
    uint64_t search_term,
    const GlobeCoordinateSparseRow* top_level_ram_index,
    uint64_t* out_abs_pointer,
    DatabaseContext* ctx
) {
    return generic_uint64_search(
        search_term,
        (const void*)top_level_ram_index,
        ctx->header.globe_search.top_level_rows,
        ctx->header.globe_search.num_sparse_levels,
        ctx->header.globe_search.chunk_size,
        ctx->header.globe_search.level_offsets,
        ctx->header.globe_search.level_sizes,
        sizeof(GlobeCoordinateSparseRow),
        sizeof(GlobeCoordinateRow),
        out_abs_pointer,
        ctx->platform
    );
}

#endif

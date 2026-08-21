#include "../common/generated_database_constants.h"

#if WIKI_PDA_ENABLE_TEMPORAL_SEARCH

#include "temporal_search.h"
#include "generic_search.h"

bool load_temporal_top_index(TemporalSparseRow** out_top_level_index, DatabasePlatform platform) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        TEMPORAL_SEARCH_TOP_LEVEL_ROWS,
        sizeof(TemporalSparseRow),
        OFFSETS_TEMPORAL_SEARCH_LEVEL[TEMPORAL_SEARCH_NUM_SPARSE_LEVELS],
        platform,
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
    DatabasePlatform platform
) {
    return generic_int64_search(
        search_term,
        (const void*)top_level_ram_index,
        TEMPORAL_SEARCH_TOP_LEVEL_ROWS,
        TEMPORAL_SEARCH_NUM_SPARSE_LEVELS,
        TEMPORAL_SEARCH_CHUNK_SIZE_ROWS,
        OFFSETS_TEMPORAL_SEARCH_LEVEL,
        sizeof(TemporalSparseRow),
        sizeof(TemporalRow),
        out_abs_pointer,
        platform
    );
}

#endif

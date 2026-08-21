#include "../common/generated_database_constants.h"

#if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH

#include "globe_coordinate_search.h"
#include "generic_search.h"

bool load_globe_coordinate_top_index(GlobeCoordinateSparseRow** out_top_level_index, DatabasePlatform platform) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        GLOBE_COORDINATE_SEARCH_TOP_LEVEL_ROWS,
        sizeof(GlobeCoordinateSparseRow),
        OFFSETS_GLOBE_COORDINATE_SEARCH_LEVEL[GLOBE_COORDINATE_SEARCH_NUM_SPARSE_LEVELS],
        platform,
        "Globe Coordinate"
    );
}

void free_globe_coordinate_top_index(GlobeCoordinateSparseRow* top_level_index) {
    free_top_level_index_generic((void*) top_level_index);
}

bool globe_coordinate_search(
    int64_t search_term,
    const GlobeCoordinateSparseRow* top_level_ram_index, 
    uint64_t* out_abs_pointer, 
    DatabasePlatform platform
) {
    return generic_int64_search(
        search_term,
        (const void*)top_level_ram_index,
        GLOBE_COORDINATE_SEARCH_TOP_LEVEL_ROWS,
        GLOBE_COORDINATE_SEARCH_NUM_SPARSE_LEVELS,
        GLOBE_COORDINATE_SEARCH_CHUNK_SIZE_ROWS,
        OFFSETS_GLOBE_COORDINATE_SEARCH_LEVEL,
        sizeof(GlobeCoordinateSparseRow),
        sizeof(GlobeCoordinateRow),
        out_abs_pointer,
        platform
    );
}

#endif

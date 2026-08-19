#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
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

void free_globe_coordinate_top_index(GlobeCoordinateSparseRow* index) {
}




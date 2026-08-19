#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "temporal_search.h"
#include "../../include/database_platform.h"

bool load_temporal_top_index(TemporalSparseRow** out_top_level_index, DatabasePlatform platform) {
    *out_top_level_index = NULL;
    return true;
}

void free_temporal_top_index(TemporalSparseRow* top_level_index) {
}

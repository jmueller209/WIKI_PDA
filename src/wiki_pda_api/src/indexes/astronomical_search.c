#include <stdlib.h> 
#include <stdio.h>
#include <string.h>
#include "astronomical_search.h"
#include "../../include/database_platform.h"


bool load_astronomical_top_index(AstronomicalSparseRow** out_top_level_index, DatabasePlatform platform) {
    *out_top_level_index = NULL;
    return true;
}

void free_astronomical_top_index(AstronomicalSparseRow* top_level_index) {
}

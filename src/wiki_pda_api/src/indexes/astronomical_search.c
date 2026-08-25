#include "../common/generated_database_constants.h"

#if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH

#include "astronomical_search.h"
#include "generic_search.h"

bool load_astronomical_top_index(AstronomicalSparseRow** out_top_level_index, DatabasePlatform platform) {
    return load_top_level_index_generic(
        (void**)out_top_level_index,
        ASTRONOMICAL_SEARCH_TOP_LEVEL_ROWS,
        sizeof(AstronomicalSparseRow),
        OFFSETS_ASTRONOMICAL_SEARCH_LEVEL[ASTRONOMICAL_SEARCH_NUM_SPARSE_LEVELS],
        platform,
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
    DatabasePlatform platform
) {
    return generic_int64_search(
        search_term,
        (const void*)top_level_ram_index,
        ASTRONOMICAL_SEARCH_TOP_LEVEL_ROWS,
        ASTRONOMICAL_SEARCH_NUM_SPARSE_LEVELS,
        ASTRONOMICAL_SEARCH_CHUNK_SIZE_ROWS,
        OFFSETS_ASTRONOMICAL_SEARCH_LEVEL,
        SIZES_ASTRONOMICAL_SEARCH_LEVEL,
        sizeof(AstronomicalSparseRow),
        sizeof(AstronomicalRow),
        out_abs_pointer,
        platform
    );
}

#endif

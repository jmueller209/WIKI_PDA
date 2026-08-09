#ifndef WIKI_DB_INTERNAL_H
#define WIKI_DB_INTERNAL_H

#include "../../include/wiki_db_api.h"
#include "../indexes/astronomical_search.h"
#include "../indexes/omni_search.h"
#include "../indexes/globe_coordinate_search.h"
#include "../indexes/temporal_search.h"

struct DatabaseContext_t {
    OmniSparseRow* omni_top_index;
    AstronomicalSparseRow* astronomical_top_index;
    TemporalSparseRow* temporal_top_index;
    GlobeCoordinateSparseRow* globe_coordinate_top_index;

    uint8_t* zstd_dict;
    uint64_t zstd_dict_length;

    DatabasePlatform platform;
};

#endif

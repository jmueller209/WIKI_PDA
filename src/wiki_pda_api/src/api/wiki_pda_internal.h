// #ifndef WIKI_DB_INTERNAL_H
// #define WIKI_DB_INTERNAL_H
//
// #include "../../include/wiki_pda.h"
// #include "../indexes/astronomical_search.h"
// #include "../indexes/omni_search.h"
// #include "../indexes/globe_coordinate_search.h"
// #include "../indexes/temporal_search.h"
// #include "../common/generated_database_constants.h"
// #include <stdio.h>
//
// #ifdef DEBUG_MODE
//     #define DEBUG_PRINT(fmt, ...) printf("[DEBUG] " fmt "\n", ##__VA_ARGS__)
// #else
//     #define DEBUG_PRINT(fmt, ...)
// #endif
//
// struct DatabaseContext_t {
//     #if WIKI_PDA_ENABLE_OMNI_SEARCH
//     OmniSparseRow* omni_top_index;
//     #endif
//
//     #if WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
//     AstronomicalSparseRow* astronomical_top_index;
//     #endif
//
//     #if WIKI_PDA_ENABLE_TEMPORAL_SEARCH
//     TemporalSparseRow* temporal_top_index;
//     #endif
//
//     #if WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
//     GlobeCoordinateSparseRow* globe_coordinate_top_index;
//     #endif
//
//     uint8_t* zstd_dict;
//     uint64_t zstd_dict_length;
//
//     DatabasePlatform platform;
// };
//
// #endif

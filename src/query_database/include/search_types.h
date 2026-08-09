#ifndef SEARCH_TYPES_H
#define SEARCH_TYPES_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>
#include "../src/common/generated_database_constants.h"


typedef enum {
    INDEX_OMNI               = (1 << 0),
    INDEX_ASTRONOMICAL       = (1 << 1),
    INDEX_TEMPORAL           = (1 << 2),
    INDEX_GLOBE_COORDINATE   = (1 << 3)
} DatabaseIndex;

typedef uint32_t DatabaseIndexMask;

// ---------------------------------------------------------
// Search Types & Tags
// ---------------------------------------------------------
typedef enum {
    SEARCH_TYPE_OMNI,
    SEARCH_TYPE_TEMPORAL,
    SEARCH_TYPE_SPATIAL,
    SEARCH_TYPE_ASTRONOMICAL,
    SEARCH_TYPE_QID, 
    SEARCH_TYPE_PID
} SearchType;

typedef uint32_t SearchTagMask;

// ---------------------------------------------------------
// Article Types
// ---------------------------------------------------------
typedef uint32_t ArticleType; 

// ---------------------------------------------------------
// Query Configuration
// ---------------------------------------------------------
typedef struct {
    SearchType type;
    
    // Clean Public Union: Just pointers and primitives!
    union {
        const char* term;         // For OMNI, TEMPORAL
        uint32_t target_qid;      // For QID
        uint32_t target_pid;      // For PID
        
        struct {
            uint32_t lat;
            uint32_t lon;
        } astronomical_term;

        struct {
            uint32_t lat;
            uint32_t lon;
        } globe_coordinate_term;
    } target;
    
    // Tag Filters (Applies to the Search Index phase, e.g., Omni)
    SearchTagMask exact_tags;
    SearchTagMask include_tags;
    SearchTagMask exclude_tags;
    
    // Article Filter (Applies to the QID data retrieval phase)
    ArticleType article_type;
} SearchQuery;


// ---------------------------------------------------------
// Results & Iteration
// ---------------------------------------------------------
typedef struct {
    uint32_t qid;
    SearchTagMask tags;
    ArticleType article_type;
    const char* title;
    // "Pointer to the data" on the SD card:
    uint64_t data_offset; 
    uint32_t data_length; 
} SearchResult;

typedef struct DatabaseContext_t DatabaseContext;
typedef struct SearchCursor_t SearchCursor;

typedef struct DataStream_t DataStream;

#endif // SEARCH_TYPES_H

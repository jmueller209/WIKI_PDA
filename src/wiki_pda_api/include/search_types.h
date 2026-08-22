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

typedef enum {
    SEARCH_TYPE_OMNI,
    SEARCH_TYPE_TEMPORAL,
    SEARCH_TYPE_GLOBE_COORDINATE,
    SEARCH_TYPE_ASTRONOMICAL,
    SEARCH_TYPE_QID, 
    SEARCH_TYPE_PID
} SearchType;

typedef uint32_t SearchTagMask;

typedef uint32_t ArticleType; 

/**
 * Defines a query for the Wikipedia PDA database.
 * 
 * Best Practice: Always initialize with {0} or via a designated initializer
 * to ensure all unused filters and sorting flags default to 0 / false.
 */
typedef struct {
    /** The specific search index to target (e.g., SEARCH_TYPE_OMNI, SEARCH_TYPE_GLOBE). */
    SearchType type;

    union {
        /** 
         * Used for SEARCH_TYPE_OMNI.
         * Pointer to a null-terminated string containing the search text. 
         * The string must remain valid in memory until search_begin() completes.
         */
        const char* omni_text;

        /** 
         * Used for SEARCH_TYPE_QID. 
         * The exact Wikidata QID to fetch (e.g., 42 for Douglas Adams).
         */
        uint32_t qid;

        /** 
         * Used for SEARCH_TYPE_PID. 
         * The exact Wikipedia Page ID to fetch.
         */
        uint32_t pid;

        /** Used for SEARCH_TYPE_ASTRONOMICAL. */
        struct {
            /** Declination in degrees (analogous to latitude on the celestial sphere). */
            double dec;
            /** Right Ascension in degrees (analogous to longitude on the celestial sphere). */
            double ra;
            /** The maximum angular distance from the target coordinates to search within. */
            float search_radius_degrees;

            // --- TOP-K SORTING ---
            /** If true, the API tracks the closest items and yields them sorted by distance. */
            bool sort_by_distance; 
            /** Maximum number of results to keep. ONLY used if sort_by_distance == true. */
            uint16_t max_results; 
        } astronomical;

        /** Used for SEARCH_TYPE_GLOBE. */
        struct {
            /** Latitude on Earth in decimal degrees. */
            double lat;
            /** Longitude on Earth in decimal degrees. */
            double lon;
            /** The physical search radius around the target coordinates. */
            float search_radius_km; 

            // --- TOP-K SORTING ---
            /** If true, the API tracks the closest items and yields them sorted by distance. */
            bool sort_by_distance;
            /** Maximum number of results to keep. ONLY used if sort_by_distance == true. */
            uint16_t max_results; 
        } globe;

        /** Used for SEARCH_TYPE_TEMPORAL. */
        struct {
            /** The central target date/time (e.g., "1969-07-20"). */
            const char* temporal_iso_string;
            /** The search range stretching into the past (e.g., an ISO duration like "P5Y" for 5 years). */
            const char* past_range_iso;
            /** The search range stretching into the future (e.g., "P1M" for 1 month). */
            const char* future_range_iso;
        } temporal;

    } target;

    // --- GLOBAL FILTERS ---
    // These filters apply to all search types. If set to 0, they are ignored.

    /** The resulting item's tags must match this mask EXACTLY. */
    SearchTagMask exact_tags;
    
    /** The resulting item MUST contain ALL tags specified in this mask. */
    SearchTagMask include_tags;
    
    /** The resulting item MUST NOT contain ANY tags specified in this mask. */
    SearchTagMask exclude_tags;

    /** Specifies what kind of the article to fetch (e.g., Metadata-only, Full Content in given language). */
    ArticleType article_type;

} SearchQuery;


typedef struct {
    uint32_t qid;
    SearchTagMask tags;
    ArticleType article_type;
    const char* title;
    const char* term;
    uint64_t data_offset; 
    uint32_t data_length; 
} SearchResult;

typedef struct DatabaseContext_t DatabaseContext;
typedef struct SearchCursor_t SearchCursor;

typedef struct DataStream_t DataStream;

#endif

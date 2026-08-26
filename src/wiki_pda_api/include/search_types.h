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

/**
 * @brief Bitmask flags representing the available database indexes.
 * Used to specify which indexes should be loaded into memory or targeted.
 */
typedef enum {
    INDEX_OMNI               = (1 << 0), /**< Text-based search index */
    INDEX_ASTRONOMICAL       = (1 << 1), /**< Celestial coordinate search index (Dec/RA) */
    INDEX_TEMPORAL           = (1 << 2), /**< Time-based search index (Dates/Years) */
    INDEX_GLOBE_COORDINATE   = (1 << 3)  /**< Earth coordinate search index (Lat/Lon) */
} DatabaseIndex;

/** @brief A bitmask combining one or more DatabaseIndex flags. */
typedef uint32_t DatabaseIndexMask;

/**
 * @brief Specifies the exact type of search to perform for a given query.
 */
typedef enum {
    SEARCH_TYPE_OMNI,
    SEARCH_TYPE_TEMPORAL,
    SEARCH_TYPE_GLOBE_COORDINATE,
    SEARCH_TYPE_ASTRONOMICAL,
    SEARCH_TYPE_QID,
    SEARCH_TYPE_PID
} SearchType;

/** @brief Bitmask used for filtering items based on assigned categorical tags. */
typedef uint32_t SearchTagMask;

/** @brief Identifier defining the payload format (e.g., Metadata-only, specific language content). */
typedef uint32_t ArticleType;

/**
 * @brief Defines a query for the Wikipedia PDA database.
 * 
 * Best Practice: Always initialize with {0} or via a designated initializer
 * to ensure all unused filters and sorting flags default to 0 / false.
 */
typedef struct {
    /** @brief The specific search index to target (e.g., SEARCH_TYPE_OMNI, SEARCH_TYPE_GLOBE). */
    SearchType type;

    union {


        /** @brief Used for SEARCH_TYPE_OMNI. */
        struct {
            /**
             * @brief Pointer to a null-terminated string containing the search text.
             * The string must remain valid in memory until search_begin() completes.
             */
            const char* text;
        } omni;

        /** Used for SEARCH_TYPE_ASTRONOMICAL. */
        struct {
            /** 
             * @brief Declination in degrees (analogous to latitude on the celestial sphere).
             * Valid range: [-90.0, +90.0] (from South Celestial Pole to North Celestial Pole).
             */
            double dec;

            /** 
             * @brief Right Ascension in degrees (analogous to longitude on the celestial sphere).
             * Valid range: [0.0, 360.0).
             */
            double ra;

            /** 
             * @brief The maximum angular distance from the target coordinates to search within.
             * Valid range: [0.0, 180.0] (180 degrees covers the entire sky from the target point).
             */
            float search_radius_degrees;

            // --- TOP-K SORTING ---
            /** @brief If true, the API tracks the closest items and yields them sorted by distance. */
            bool sort_by_distance;
            /** @brief Maximum number of results to keep. ONLY used if sort_by_distance == true. */
            uint16_t max_results;
        } astronomical;

        /** @brief Used for SEARCH_TYPE_GLOBE. */
        struct {
            /** 
             * @brief Latitude on Earth in decimal degrees.
             * Valid range: [-90.0, +90.0] (from South Pole to North Pole).
             */
            double lat;

            /** 
             * @brief Longitude on Earth in decimal degrees.
             * Valid range: [-180.0, +180.0] (from West to East relative to the Prime Meridian).
             */
            double lon;

            /** 
             * @brief The physical search radius around the target coordinates in kilometers.
             * Valid range: [0.0, ~20015.0] (20015 km represents roughly half the Earth's circumference, covering the entire globe).
             */
            float search_radius_km;

            // --- TOP-K SORTING ---
            /** @brief If true, the API tracks the closest items and yields them sorted by distance. */
            bool sort_by_distance;
            /** @brief Maximum number of results to keep. ONLY used if sort_by_distance == true. */
            uint16_t max_results;
        } globe;

        /** @brief Used for SEARCH_TYPE_TEMPORAL. */
        struct {
            /** 
             * @brief The central target date encoded as a chronologically sortable integer.
             * The format must strictly follow: (Year * 10000) + (Month * 100) + Day.
             * Negative years represent dates BC (Before Christ).
             * Examples: 19690720 (July 20, 1969) or -5000101 (January 1st, 500 BC).
             */
            int64_t date_code;
            /**
             * @brief If true, search results will follow chronological order (into the future),
             * otherwise the API searches backwards in time (into the past).
             */
            bool search_forward;
        } temporal;

        /** @brief Used for SEARCH_TYPE_QID. */
        struct {
            /** @brief The exact Wikidata QID (Item ID) to target (e.g., 42 for Q42). */
            uint64_t id;
            /** @brief If true, subsequent search_next() calls will traverse QIDs in ascending order. If false, descending. */
            bool search_forward;
            /** 
             * @brief If true, the search will immediately fail if the exact 'id' does not exist.
             * If false, the cursor acts as a pager, automatically snapping to the nearest valid QID in the search direction.
             */
            bool first_result_must_match;
        } qid;

        /** @brief Used for SEARCH_TYPE_PID. */
        struct {
            /** @brief The exact Wikidata PID (Property ID) to target (e.g., 31 for P31). */
            uint64_t id;
            /** @brief If true, subsequent search_next() calls will traverse PIDs in ascending order. If false, descending. */
            bool search_forward;
            /** 
             * @brief If true, the search will immediately fail if the exact 'id' does not exist.
             * If false, the cursor acts as a pager, automatically snapping to the nearest valid PID in the search direction.
             */
            bool first_result_must_match;
        } pid;

    } target;

    // --- GLOBAL FILTERS ---
    // These filters apply to all search types. If set to 0, they are ignored.

    /** @brief The resulting item's tags must match this mask EXACTLY. */
    SearchTagMask exact_tags;

    /** @brief The resulting item MUST contain ALL tags specified in this mask. */
    SearchTagMask include_tags;

    /** @brief The resulting item MUST NOT contain ANY tags specified in this mask. */
    SearchTagMask exclude_tags;

    /** @brief Specifies what kind of the article to fetch (e.g., Metadata-only, Full Content in given language). */
    ArticleType article_type;

} SearchQuery;

/**
 * @brief Represents a single matched item returned by the database engine.
 */
typedef struct {
    uint32_t qid;               /**< The Wikidata ID of the matched item. */
    SearchTagMask tags;         /**< The category tags associated with this item. */
    ArticleType article_type;   /**< The type of payload available at the data offset. */
    const char* title;          /**< Pointer to a temporary buffer holding the article title. */
    const char* term;           /**< Pointer to a temporary buffer holding the matched term/coordinate/date. */
    uint64_t data_offset;       /**< Absolute physical offset in the database file for this item's payload. */
    uint32_t data_length;       /**< Size in bytes of the compressed payload. */
} SearchResult;

// --- OPAQUE HANDLES ---
/** @brief Opaque handle representing an open connection to the database. */
typedef struct DatabaseContext_t DatabaseContext;

/** @brief Opaque handle representing an active search iteration state. */
typedef struct SearchCursor_t SearchCursor;

/** @brief Opaque handle for reading compressed payload data chunks. */
typedef struct DataStream_t DataStream;

#endif

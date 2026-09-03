#ifndef SEARCH_TYPES_H
#define SEARCH_TYPES_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>

#ifdef __cplusplus
extern "C" {
#endif

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
    uint32_t exact_tags;

    /** @brief The resulting item MUST contain ALL tags specified in this mask. */
    uint32_t include_tags;

    /** @brief The resulting item MUST NOT contain ANY tags specified in this mask. */
    uint32_t exclude_tags;

    /** @brief Specifies what kind of the article to fetch (e.g., Metadata-only, Full Content in given language). */
    uint32_t article_type;

} SearchQuery;

/**
 * @brief Represents a single matched item returned by the database engine.
 */
typedef struct {
    uint32_t id;                /**< The Wikidata ID of the matched item. (QID or PID based on search type) */
    uint32_t tags;         /**< The category tags associated with this item. */
    uint32_t article_type;   /**< The type of payload available at the data offset. */
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

/**
 * @brief Supported Wikipedia Projects/Languages
 */
typedef enum {
    WPDA_METADATA = 0,      // Meta data
    WPDA_LANG_EN = 1,       // English
    WPDA_LANG_CEB = 2,      // Cebuano
    WPDA_LANG_DE = 3,       // German
    WPDA_LANG_SV = 4,       // Swedish
    WPDA_LANG_FR = 5,       // French
    WPDA_LANG_NL = 6,       // Dutch
    WPDA_LANG_RU = 7,       // Russian
    WPDA_LANG_ES = 8,       // Spanish
    WPDA_LANG_IT = 9,       // Italian
    WPDA_LANG_PL = 10,      // Polish
    WPDA_LANG_JA = 11,      // Japanese
    WPDA_LANG_ZH = 12,      // Chinese
    WPDA_LANG_VI = 13,      // Vietnamese
    WPDA_LANG_UK = 14,      // Ukrainian
    WPDA_LANG_AR = 15,      // Arabic
    WPDA_LANG_PT = 16,      // Portuguese
    WPDA_LANG_FA = 17,      // Persian
    WPDA_LANG_CA = 18,      // Catalan
    WPDA_LANG_SR = 19,      // Serbian
    WPDA_LANG_ID = 20,      // Indonesian
    WPDA_LANG_KO = 21,      // Korean
    WPDA_LANG_NO = 22,      // Norwegian
    WPDA_LANG_FI = 23,      // Finnish
    WPDA_LANG_TR = 24,      // Turkish
    WPDA_LANG_HU = 25,      // Hungarian
    WPDA_LANG_CS = 26,      // Czech
    WPDA_LANG_RO = 27,      // Romanian
    WPDA_LANG_EU = 28,      // Basque
    WPDA_LANG_MS = 29,      // Malay
    WPDA_LANG_EO = 30,      // Esperanto
    WPDA_LANG_HE = 31,      // Hebrew
    WPDA_LANG_DA = 32,      // Danish
    WPDA_LANG_BG = 33,      // Bulgarian
    WPDA_LANG_SK = 34,      // Slovak
    WPDA_LANG_ET = 35,      // Estonian
    WPDA_LANG_BE = 36,      // Belarusian
    WPDA_LANG_SIMPLE = 37,  // Simple English
    WPDA_LANG_EL = 38,      // Greek
    WPDA_LANG_HR = 39,      // Croatian
    WPDA_LANG_LT = 40,      // Lithuanian
    WPDA_LANG_GL = 41,      // Galician
    WPDA_LANG_SL = 42,      // Slovenian
    WPDA_LANG_UR = 43,      // Urdu
    WPDA_LANG_HI = 44,      // Hindi
    WPDA_LANG_TH = 45,      // Thai
    WPDA_LANG_BN = 46,      // Bengali
    WPDA_LANG_TA = 47,      // Tamil
    WPDA_LANG_TE = 48,      // Telugu
    WPDA_LANG_SW = 49,      // Swahili
    WPDA_LANG_LV = 50       // Latvian
} WPDA_Project;

/**
 * @brief 32-bit Bitmasks for Wikidata Entity Tags
 * Used to filter search indexes. Because these are bitmasks, 
 * multiple tags can be combined using the bitwise OR (|) operator.
 */
typedef enum {
    // People
    WPDA_TAG_HUMAN_Q5               = (1 << 0),

    // Geography & Places
    WPDA_TAG_CITY_Q515              = (1 << 1),
    WPDA_TAG_CAPITAL_CITY_Q5119     = (1 << 2),
    WPDA_TAG_COUNTRY_Q6256          = (1 << 3),
    WPDA_TAG_SETTLEMENT_Q486972     = (1 << 4),
    WPDA_TAG_MOUNTAIN_Q8502         = (1 << 5),
    WPDA_TAG_RIVER_Q4022            = (1 << 6),

    // Media, Art & Tech
    WPDA_TAG_FILM_Q11424            = (1 << 7),
    WPDA_TAG_LITERARY_WORK_Q7725634 = (1 << 8),
    WPDA_TAG_BOOK_Q571              = (1 << 9),
    WPDA_TAG_ALBUM_Q482994          = (1 << 10),
    WPDA_TAG_VIDEO_GAME_Q1194951    = (1 << 11),

    // Society & History
    WPDA_TAG_COMPANY_Q783794        = (1 << 12),
    WPDA_TAG_ORGANIZATION_Q43229    = (1 << 13),

    // Biology
    WPDA_TAG_TAXON_Q16521           = (1 << 14),

    // Events
    WPDA_TAG_EVENT_Q1190554         = (1 << 15),

    // Astronomy & Space
    WPDA_TAG_STAR_Q523              = (1 << 16),
    WPDA_TAG_GALAXY_Q318            = (1 << 17),
    WPDA_TAG_PLANET_Q634            = (1 << 18),
    WPDA_TAG_MOON_Q2537             = (1 << 19),
    WPDA_TAG_NEBULA_Q3559           = (1 << 20),
    WPDA_TAG_MINOR_PLANET_Q1022867  = (1 << 21),
    WPDA_TAG_ASTEROID_Q3863         = (1 << 22)
} WPDA_TagMask;

#ifdef __cplusplus
}
#endif

#endif

/**
 * @file wiki_pda_options.h
 * @brief User-configurable options for the Wiki PDA library.
 *
 * This file contains macros that tune the memory footprint and performance
 * of the database queries. You can modify these values directly here or
 * override them via your build system (e.g., using -D compiler flags)
 * without altering the library source code.
 */

#ifndef WIKI_PDA_OPTIONS_H
#define WIKI_PDA_OPTIONS_H

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// FEATURE TOGGLES (INDEX ENABLES)
// ============================================================================
// You can disable specific search indexes by setting these to 0. 
// Disabling unused indexes prevents the associated code from being compiled,
// significantly reducing the flash memory size of your final firmware. This is 
// highly recommended for constrained embedded devices (like ESP32 or Teensy).

/**
 * @brief Enable or disable the Omni (Text) Search functionality.
 * Set to 1 to enable, 0 to disable.
 */
#ifndef WIKI_PDA_ENABLE_OMNI_SEARCH
#define WIKI_PDA_ENABLE_OMNI_SEARCH 1
#endif

/**
 * @brief Enable or disable the Temporal (Time/Date) Search functionality.
 * Set to 1 to enable, 0 to disable.
 */
#ifndef WIKI_PDA_ENABLE_TEMPORAL_SEARCH
#define WIKI_PDA_ENABLE_TEMPORAL_SEARCH 1
#endif

/**
 * @brief Enable or disable the Astronomical (Celestial Coordinates) Search functionality.
 * Set to 1 to enable, 0 to disable.
 */
#ifndef WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH
#define WIKI_PDA_ENABLE_ASTRONOMICAL_SEARCH 1
#endif

/**
 * @brief Enable or disable the Globe (Earth Coordinates) Search functionality.
 * Set to 1 to enable, 0 to disable.
 */
#ifndef WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH
#define WIKI_PDA_ENABLE_GLOBE_COORDINATE_SEARCH 1
#endif

// ============================================================================
// OMNI SEARCH OPTIONS
// ============================================================================

/**
 * @brief Size of the QID deduplication cache.
 *
 * Used to make sure QIDs are not considered multiple times when searching
 * (specifically for the omni search index). A larger cache prevents duplicate
 * results but consumes more stack/heap memory.
 */
#ifndef MAX_DEDUPLICATION_CACHE
#define MAX_DEDUPLICATION_CACHE 128
#endif

// ============================================================================
// SPATIAL & ASTRONOMICAL SEARCH OPTIONS
// ============================================================================

/**
 * @brief Maximum capacity for sorted spatial search results.
 *
 * Defines the maximum number of results saved when performing sorted spatial
 * queries. This is relevant for both sorted astronomical search and sorted
 * globe coordinate search.
 */
#ifndef MAX_SORTED_RESULTS
#define MAX_SORTED_RESULTS 50
#endif

/**
 * @brief Maximum number of Morton ranges to search.
 *
 * A lower number reduces the number of random reads the database needs to perform,
 * saving overhead on storage media with high latency (like SD cards).
 * Increasing this number will increase the number of sequential reads needed.
 * Relevant for astronomical and globe coordinate search.
 */
#ifndef MAX_MORTON_RANGES
#define MAX_MORTON_RANGES 64
#endif

/**
 * @brief Number of bytes used to encode term in omni search index.
 *
 * This needs to be of the form "2^x - 8" where x is a whole number
 * and be compatible with the 'omni_search_index_term_encoding_bytes'
 * settings in the config file of the database generator. Do not touch this unless
 * you know what you are doing.
 */
#ifndef OMNI_SEARCH_TERM_SIZE
#define OMNI_SEARCH_TERM_SIZE 24
#endif

#ifdef __cplusplus
}
#endif

#endif // WIKI_PDA_OPTIONS_H

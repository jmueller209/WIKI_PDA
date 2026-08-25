#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include "../include/wiki_pda.h"

// ============================================================================
// PROFILING CONFIGURATION
// Change these values to test different parts of your database engine
// ============================================================================

// --- GENERAL SETTINGS ---
// 1 = Omni Search (Text)
// 2 = Globe Coordinate Search (Lat/Lon)
// 3 = Astronomical Search (Dec/RA)
#define TEST_SEARCH_TYPE 2

#define TEST_MAX_RESULTS 50
#define TEST_SORT_BY_DISTANCE 0 // 1 = Top-K mode, 0 = Fast Stream mode
#define TEST_ARTICLE_TYPE 1     // Usually 1 for standard articles

// --- OMNI SEARCH SETTINGS ---
#define TEST_OMNI_TERM "berlin"

// --- SPATIAL SEARCH SETTINGS (Globe & Astro) ---
#define TEST_LAT_DEC 0.0f
#define TEST_LON_RA 0.0f
#define TEST_RADIUS 40000.0f // Kilometers for Globe, Degrees for Astro

// ============================================================================

int main(int argc, char** argv) {
    printf("Starting automated profiling run...\n");

    // 1. Open Database
    FILE* db_file = fopen("bin/data_base.bin", "rb");
    if (!db_file) {
        printf("ERROR: Could not open bin/data_base.bin\n");
        return 1;
    }

    // platform_desktop is loaded automatically from src/platforms/desktop.c
    DatabasePlatform pc_platform = platform_desktop(db_file);
    DatabaseIndexMask mask = INDEX_OMNI | INDEX_GLOBE_COORDINATE | INDEX_ASTRONOMICAL | INDEX_TEMPORAL;
    DatabaseContext* ctx = db_init(mask, pc_platform);

    if (ctx == NULL) {
        printf("ERROR: db_init failed.\n");
        fclose(db_file);
        return 1;
    }

    // 2. Construct the Query from Macros
    SearchQuery query;
    memset(&query, 0, sizeof(SearchQuery));
    query.article_type = TEST_ARTICLE_TYPE;

#if TEST_SEARCH_TYPE == 1
    query.type = SEARCH_TYPE_OMNI;
    query.target.omni_text = TEST_OMNI_TERM;
    printf("Config: Omni Search -> '%s'\n", TEST_OMNI_TERM);

#elif TEST_SEARCH_TYPE == 2
    query.type = SEARCH_TYPE_GLOBE_COORDINATE;
    query.target.globe.lat = TEST_LAT_DEC;
    query.target.globe.lon = TEST_LON_RA;
    query.target.globe.search_radius_km = TEST_RADIUS;
    query.target.globe.sort_by_distance = (TEST_SORT_BY_DISTANCE == 1);
    query.target.globe.max_results = TEST_MAX_RESULTS;
    printf("Config: Globe Search -> Lat: %.2f, Lon: %.2f, Radius: %.2f km, Top-K: %d\n", 
           TEST_LAT_DEC, TEST_LON_RA, TEST_RADIUS, TEST_SORT_BY_DISTANCE);

#elif TEST_SEARCH_TYPE == 3
    query.type = SEARCH_TYPE_ASTRONOMICAL;
    query.target.astronomical.dec = TEST_LAT_DEC;
    query.target.astronomical.ra = TEST_LON_RA;
    query.target.astronomical.search_radius_degrees = TEST_RADIUS;
    query.target.astronomical.sort_by_distance = (TEST_SORT_BY_DISTANCE == 1);
    query.target.astronomical.max_results = TEST_MAX_RESULTS;
    printf("Config: Astro Search -> Dec: %.2f, RA: %.2f, Radius: %.2f deg, Top-K: %d\n", 
           TEST_LAT_DEC, TEST_LON_RA, TEST_RADIUS, TEST_SORT_BY_DISTANCE);
#else
    printf("ERROR: Invalid TEST_SEARCH_TYPE\n");
    return 1;
#endif

    // 3. Execute the Search
    SearchCursor* cursor = search_begin(ctx, &query);
    if (cursor == NULL) {
        printf("Search initialization failed or index empty.\n");
        db_end(ctx);
        fclose(db_file);
        return 1;
    }

    SearchResult result;
    int match_count = 0;

    // We suppress the heavy printf() statements inside the loop 
    // so they don't skew the CPU profiling metrics.
    while (search_next(cursor, &result)) {
        match_count++;
        if (match_count >= TEST_MAX_RESULTS) break;
    }

    printf("Search completed. Found %d results.\n", match_count);

    // 4. Cleanup
    search_end(cursor);
    db_end(ctx);
    fclose(db_file);
    
    printf("Profiling run finished cleanly.\n");
    return 0;
}

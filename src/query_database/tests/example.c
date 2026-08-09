#include <stdio.h>
#include <stdbool.h>
#include <stdint.h>

#include "../include/wiki_db_api.h"

// We expose the global file pointer from wiki_db_api.c so we can open it here
extern FILE* g_database_file;

int main() {
    // 1. Setup the mock SD card environment
    g_database_file = fopen("bin/data_base.bin", "rb");
    if (g_database_file == NULL) {
        return 1;
    }

    // 2. Initialize the Database Context (Loading Omni Index and Globe Coordinate Index)
    DatabaseContext* ctx = db_init(INDEX_OMNI ||  INDEX_GLOBE_COORDINATE);
    if (ctx == NULL) {
        return 1;
    }

    // 3. Create search query
    SearchQuery query = {0}; // {0} ensures all bitmasks default to 0
    query.type = SEARCH_TYPE_OMNI; // specify which index to search
    query.target.term = "Unive"; // Search term 
    query.article_type = 1; // Assuming 1 = Wikipedia English (Check your database)

    // 4. Being the search by creating the search cursor (iterator)
    SearchCursor* cursor = search_begin(ctx, &query);
    if (cursor != NULL) {
    	
	// 5. Create the result objects
	SearchResult result;

	// 6. Perform the search
    int match_count = 0;
	while (search_next(cursor, &result)) {
            printf("[%d] Title: %.*s\n", match_count, OMNI_SEARCH_TERM_SIZE, result.title);
            printf("    QID: Q%u | Tags: %u | Type: %u\n", result.qid, result.tags, result.article_type);
            printf("    Data Offset: %llu | Data Length: %u bytes\n", (unsigned long long)result.data_offset, result.data_length);
            printf("------------------------\n");
            match_count++;
            if (match_count >= 10) {
                break;
            }
        }
    }
   return 0;
}

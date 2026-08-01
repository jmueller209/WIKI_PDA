#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../omni_search.h"

// ---------------------------------------------------------
// 1. PLATFORM IMPLEMENTATION (PC)
// ---------------------------------------------------------
// This replaces the need for passing void* around. 
// The pure search logic calls this, and we implement it using PC <stdio.h>.

FILE* g_database_file = NULL; // Global file pointer for this platform

bool platform_database_read(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes) {
    if (g_database_file == NULL) return false;
    if (fseeko(g_database_file, absolute_offset, SEEK_SET) != 0) {
        return false;
    }
    return fread(buffer, 1, num_bytes, g_database_file) == num_bytes;
}

// ---------------------------------------------------------
// 3. MAIN TEST LOOP
// ---------------------------------------------------------
int main(int argc, char *argv[]) {
    // If the user provided an argument, use it. Otherwise, fallback to the default.
    const char* db_filename = (argc > 1) ? argv[1] : "database.bin";

    // 1. Open the file using the dynamic filename
    g_database_file = fopen(db_filename, "rb");
    if (!g_database_file) {
        printf("Error: Could not open '%s'.\n", db_filename);
        if (argc == 1) {
            printf("Usage: %s [path_to_database.bin]\n", argv[0]);
        }
        return 1;
    }

    // 2. Load the top level into RAM
    OmniSparseRow* ram_index = load_top_level_index();
    if (!ram_index && OMNI_SEARCH_TOP_LEVEL_ROWS > 0) {
        printf("Error: Failed to load top level index into RAM.\n");
        fclose(g_database_file);
        return 1;
    }

    printf("Database loaded successfully! (RAM used: %u bytes)\n", 
            (uint32_t)(OMNI_SEARCH_TOP_LEVEL_ROWS * sizeof(OmniSparseRow)));

    // 3. Setup the interactive loop
    char input_buffer[256];
    const uint32_t MAX_RESULTS = 10;
    OmniRow results[MAX_RESULTS];

    while (1) {
        printf("\nSearch > ");
        
        // Read input from the terminal
        if (!fgets(input_buffer, sizeof(input_buffer), stdin)) {
            break; // Exit if EOF (Ctrl+D)
        }

        // C's fgets includes the newline character (\n) when you press Enter.
        // We MUST strip it off, or it will become part of the search query!
        input_buffer[strcspn(input_buffer, "\r\n")] = '\0';

        // Check for exit commands
        if (strcmp(input_buffer, "quit") == 0 || strcmp(input_buffer, "exit") == 0) {
            break;
        }

        if (strlen(input_buffer) == 0) continue; // Skip empty inputs

        // 4. Run the pure search logic
        uint32_t match_count = omni_search(input_buffer, ram_index, results, MAX_RESULTS);

        // 5. Print out what we found
        if (match_count == 0) {
            printf("  No matches found.\n");
        } else {
            printf("  Found %u matches:\n", match_count);
            for (uint32_t i = 0; i < match_count; i++) {
                printf("  %d. Term: '%s' | QID: %u | Tags: %u\n", 
                    i + 1, 
                    results[i].term, 
                    results[i].qid, 
                    results[i].tags
                );
            }
        }
    }

    // 6. Be a good janitor and clean up
    if (ram_index) {
        free(ram_index);
    }
    fclose(g_database_file);
    printf("Goodbye!\n");
    
    return 0;
}

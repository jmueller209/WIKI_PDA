#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "../qid_search.h"
#include <inttypes.h>


FILE* g_database_file = NULL; 

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


    char input_buffer[16];
    while(1){
        printf("\nSearch > ");
        if (!fgets(input_buffer, sizeof(input_buffer), stdin)) {
            break; // Exit if EOF (Ctrl+D)
        }

        input_buffer[strcspn(input_buffer, "\r\n")] = '\0';

        // Check for exit commands
        if (strcmp(input_buffer, "quit") == 0 || strcmp(input_buffer, "exit") == 0) {
            break;
        }
        // Convert the input to an integer
        char *endptr;
        uint32_t qid = (uint32_t)strtoul(input_buffer, &endptr, 10);

        // Check if conversion failed (either not a number, or empty string)
        if (endptr == input_buffer || *endptr != '\0') {
            printf("Invalid QID. Please enter a valid integer.\n");
            continue;
        }
        IndexRow* index_rows = NULL;
        uint16_t num_rows = 0;
        if (!get_all_index_rows_for_qid(qid, &index_rows, &num_rows)) {
            printf("Error: Failed to retrieve index rows for QID %u.\n", qid);
            continue;
        }
        printf("Found %u index rows for QID %u:\n", num_rows, qid);
        for (uint16_t i = 0; i < num_rows; i++) {
            printf("Index Row %d: offset='%" PRIu64 "', length=%u, project ID=%u\n", 
                i,
                index_rows[i].offset,
                index_rows[i].length,
                index_rows[i].project_id
            );
        }

    }

    return 0;
}

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>
#include "../qid_search.h"
#include "../data_search.h"
#include "../decompress.h"


FILE* g_database_file = NULL; 

bool platform_database_read(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes) {
    if (g_database_file == NULL) return false;
    if (fseeko(g_database_file, absolute_offset, SEEK_SET) != 0) {
        return false;
    }
    return fread(buffer, 1, num_bytes, g_database_file) == num_bytes;
}

int main(int argc, char *argv[]) {

    const char* db_filename = (argc > 1) ? argv[1] : "database.bin";

    g_database_file = fopen(db_filename, "rb");
    if (!g_database_file) {
        printf("Error: Could not open '%s'.\n", db_filename);
        if (argc == 1) {
            printf("Usage: %s [path_to_database.bin]\n", argv[0]);
        }
        return 1;
    }

    uint8_t* zstd_dict = NULL;
    uint64_t zstd_dict_length = 0;
    if (!load_zstd_dictionary(&zstd_dict, &zstd_dict_length)) {
        printf("Error: Could not load ZSTD dictionary from database.\n");
        fclose(g_database_file);
        return 1;
    }
    printf("Loaded ZSTD dictionary (%" PRIu64 " bytes).\n", zstd_dict_length);


    char input_buffer[16];
    while(1){
        printf("\nSearch > ");
        if (!fgets(input_buffer, sizeof(input_buffer), stdin)) {
            break;
        }

        input_buffer[strcspn(input_buffer, "\r\n")] = '\0';

        if (strcmp(input_buffer, "quit") == 0 || strcmp(input_buffer, "exit") == 0) {
            break;
        }
        char *endptr;
        uint32_t qid = (uint32_t)strtoul(input_buffer, &endptr, 10);

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
	
	printf("\nEnter the row index to view data (or press Enter to search a new QID): ");
	char row_input[16];
        if (!fgets(row_input, sizeof(row_input), stdin)) {
            free(index_rows);
	    break;
        }

	row_input[strcspn(row_input, "\r\n")] = '\0';

        if (strlen(row_input) == 0) {
            free(index_rows); 
            continue;
        }

	char *row_endptr;
        uint32_t chosen_index = (uint32_t)strtoul(row_input, &row_endptr, 10);

        if (row_endptr == row_input || *row_endptr != '\0' || chosen_index >= num_rows) {
            printf("Invalid selection. Going back to search...\n");
            free(index_rows);
            continue;
        }

        uint8_t* compressed_data = NULL;
        char* my_text = NULL;
        uint32_t uncompressed_length = 0;

        uint64_t target_offset = index_rows[chosen_index].offset;
        uint32_t target_length = index_rows[chosen_index].length;

        if (get_data(target_offset, target_length, &compressed_data, false)) {

            if (decompress_data(compressed_data, target_length, zstd_dict, zstd_dict_length, &my_text, &uncompressed_length)) {

                printf("\n=== DATA FOR ROW %u ===\n", chosen_index);
                printf("%s\n", my_text);
                printf("=======================\n");

                free(my_text); 
            } else {
                printf("Error: Failed to decompress data.\n");
            }

            free(compressed_data);

        } else {
            printf("Error: Failed to read data from database.\n");
        }

        free(index_rows);
    }

    return 0;
}

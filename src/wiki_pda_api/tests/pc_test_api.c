#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include "../include/wiki_pda.h"

bool get_input(const char* prompt, char* buffer, size_t max_len) {
    printf("%s", prompt);
    if (fgets(buffer, max_len, stdin) == NULL) return false;
    
    buffer[strcspn(buffer, "\r\n")] = 0;

    if (strcmp(buffer, "q") == 0 || strcmp(buffer, "quit") == 0 || strcmp(buffer, "exit") == 0) {
        return false;
    }
    return true;
}

void str_to_lowercase(char* str) {
    for (int i = 0; str[i]; i++) {
        str[i] = tolower((unsigned char)str[i]);
    }
}

void read_article(DatabaseContext* ctx, SearchResult* selected) {
    if (selected->data_length == 0) {
        printf("Empty article.\n");
        return;
    }

    uint32_t current_scroll_offset = 0;
    const uint32_t VIEW_WINDOW_SIZE = 2048; 
    char esp32_screen_buffer[2048 + 1];

    while (true) {
        DataStream* stream = data_stream_begin(ctx, selected->data_offset, selected->data_length);
        if (stream == NULL) {
            printf("Failed to open stream.\n");
            break;
        }

        uint32_t bytes_skipped = 0;
        char dummy_drain[256];
        uint32_t temp_read = 0;

        while (bytes_skipped < current_scroll_offset) {
            uint32_t chunk_target = sizeof(dummy_drain);
            if (current_scroll_offset - bytes_skipped < chunk_target) {
                chunk_target = current_scroll_offset - bytes_skipped;
            }
            if (!data_stream_read(stream, dummy_drain, chunk_target, &temp_read) || temp_read == 0) break;
            bytes_skipped += temp_read;
        }

        uint32_t total_read = 0;
        while (total_read < VIEW_WINDOW_SIZE) {
            uint32_t to_read = VIEW_WINDOW_SIZE - total_read;
            uint32_t chunk_bytes = 0;
            if (!data_stream_read(stream, esp32_screen_buffer + total_read, to_read, &chunk_bytes) || chunk_bytes == 0) break;
            total_read += chunk_bytes;
        }

        esp32_screen_buffer[total_read] = '\0';
        data_stream_end(stream);

        printf("\033[H\033[J"); // Clear terminal screen
        printf("=== READING Q%u ===\n\n", selected->qid);

        if (total_read > 0) {
            printf("%s\n", esp32_screen_buffer);
        } else {
            printf("[End of article reached]\n");
        }

        printf("\n==========================================================\n");

        char cmd_buf[16];
        if (!get_input("[Enter]: Scroll down | [p]: Scroll up | [q]: Exit reader\nCommand: ", cmd_buf, sizeof(cmd_buf))) {
            break; 
        }

        if (strcmp(cmd_buf, "p") == 0 || strcmp(cmd_buf, "P") == 0) {
            if (current_scroll_offset >= 256) current_scroll_offset -= 256;
            else current_scroll_offset = 0;
        } else {
            if (total_read > 0) current_scroll_offset += 256;
        }
    }
}

bool build_query(SearchQuery* query) {
    static char input_buffer[256];

    printf("\n--- SELECT SEARCH INDEX ---\n");
    printf("1. Omni Search (Text)\n");
    printf("2. Globe Coordinate Search (Lat/Lon)\n");
    printf("3. Astronomical Search (Dec/RA)\n");
    printf("4. Temporal Search (Date)\n");

    if (!get_input("Choice (1-4) or 'q' to quit: ", input_buffer, sizeof(input_buffer))) return false;

    int choice = atoi(input_buffer);
    memset(query, 0, sizeof(SearchQuery));
    query->article_type = 1;

    switch (choice) {
        case 1:
            query->type = SEARCH_TYPE_OMNI;
            if (!get_input("Enter search term: ", input_buffer, sizeof(input_buffer))) return false;
            str_to_lowercase(input_buffer);
            query->target.omni_search_term = input_buffer;
            break;

        case 2:
            query->type = SEARCH_TYPE_GLOBE_COORDINATE;
            if (!get_input("Enter Latitude: ", input_buffer, sizeof(input_buffer))) return false;
            query->target.globe_coordinate_search_term.lat = atof(input_buffer);
            if (!get_input("Enter Longitude: ", input_buffer, sizeof(input_buffer))) return false;
            query->target.globe_coordinate_search_term.lon = atof(input_buffer);
            break;

        case 3:
            query->type = SEARCH_TYPE_ASTRONOMICAL;
            if (!get_input("Enter Declination: ", input_buffer, sizeof(input_buffer))) return false;
            query->target.astronomical_search_term.dec = atof(input_buffer);
            if (!get_input("Enter Right Ascension: ", input_buffer, sizeof(input_buffer))) return false;
            query->target.astronomical_search_term.ra = atof(input_buffer);
            break;

        case 4:
            query->type = SEARCH_TYPE_TEMPORAL;
            if (!get_input("Enter Date (e.g. 1969-07-20 or -500-01-01): ", input_buffer, sizeof(input_buffer))) return false;
            query->target.temporal_iso_string = input_buffer;
            break;

        default:
            printf("Invalid choice.\n");
            return build_query(query);
    }
    return true;
}

int execute_and_display_search(DatabaseContext* ctx, SearchQuery* query, SearchResult* displayed_results) {
    SearchCursor* cursor = search_begin(ctx, query);
    if (cursor == NULL) {
        printf("Search initialization failed or index empty.\n");
        return 0;
    }

    SearchResult result;
    int match_count = 0;

    printf("\n--- SEARCH RESULTS ---\n");

    while (search_next(cursor, &result)) {
        if (match_count < 10) {
            displayed_results[match_count] = result;
        }
        match_count++;

        printf("[%d] Match: %.*s\n", match_count, OMNI_SEARCH_TERM_SIZE, result.title);
        printf("    QID: Q%u | Length: %u bytes\n", result.qid, result.data_length);
        printf("------------------------\n");

        if (match_count >= 10) break;
    }

    search_end(cursor);
    
    if (match_count == 0) {
        printf("No matches found.\n");
    }
    return match_count;
}

int main(int argc, char** argv) {
    printf("--- ESP32 Memory-Constrained Simulation ---\n");

    FILE* db_file = fopen("bin/data_base.bin", "rb");
    if (!db_file) {
        printf("ERROR: Could not open bin/data_base.bin\n");
        return 1;
    }

    DatabasePlatform pc_platform = platform_desktop(db_file);

    DatabaseIndexMask mask = INDEX_OMNI | INDEX_GLOBE_COORDINATE | INDEX_ASTRONOMICAL | INDEX_TEMPORAL;
    DatabaseContext* ctx = db_init(mask, pc_platform);

    if (ctx == NULL) {
        printf("ERROR: db_init failed.\n");
        fclose(db_file);
        return 1;
    }

    while (true) {
        SearchQuery query;
        if (!build_query(&query)) {
            break;        }

        SearchResult displayed_results[10];
        int match_count = execute_and_display_search(ctx, &query, displayed_results);

        if (match_count == 0) continue;

        while (true) {
            char choice_str[16];
            printf("\nEnter result number to read (1-%d), or 0 for new search: ", match_count);

            if (!get_input("", choice_str, sizeof(choice_str))) break;            if (strlen(choice_str) == 0) continue;

            int choice = atoi(choice_str);
            if (choice == 0) break;

            if (choice > 0 && choice <= match_count) {
                read_article(ctx, &displayed_results[choice - 1]);
            } else {
                printf("Invalid choice.\n");
            }
        }
    }

    printf("Shutting down database...\n");
    db_end(ctx);
    fclose(db_file);
    return 0;
}

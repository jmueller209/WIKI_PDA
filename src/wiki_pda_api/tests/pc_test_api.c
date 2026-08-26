#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <inttypes.h>
#include "../include/wiki_pda.h"

#define MAX_DISPLAY_RESULTS 50

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

        printf("\033[H\033[J");
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

bool build_query(SearchQuery* query, int* out_max_results) {
    static char temp_input[256];
    static char omni_string_buffer[256];

    printf("\n--- SELECT SEARCH INDEX ---\n");
    printf("1. Omni Search (Text)\n");
    printf("2. Globe Coordinate Search (Lat/Lon)\n");
    printf("3. Astronomical Search (Dec/RA)\n");
    printf("4. Temporal Search (Date)\n");
    printf("5. QID Search (Direct ID)\n");

    if (!get_input("Choice (1-5) or 'q' to quit: ", temp_input, sizeof(temp_input))) return false;

    int choice = atoi(temp_input);
    memset(query, 0, sizeof(SearchQuery));
    query->article_type = 1;

    switch (choice) {
        case 1:
            query->type = SEARCH_TYPE_OMNI;
            if (!get_input("Enter search term: ", omni_string_buffer, sizeof(omni_string_buffer))) return false;
            str_to_lowercase(omni_string_buffer);
            query->target.omni.text = omni_string_buffer;
            break;

        case 2:
            query->type = SEARCH_TYPE_GLOBE_COORDINATE;
            if (!get_input("Enter Latitude: ", temp_input, sizeof(temp_input))) return false;
            query->target.globe.lat = atof(temp_input);

            if (!get_input("Enter Longitude: ", temp_input, sizeof(temp_input))) return false;
            query->target.globe.lon = atof(temp_input);

            if (!get_input("Enter search radius (km): ", temp_input, sizeof(temp_input))) return false;
            query->target.globe.search_radius_km = (float)atof(temp_input);

            if (!get_input("Sort by distance? (1 = Yes [Top-K], 0 = No [Fast Stream]): ", temp_input, sizeof(temp_input))) return false;
            query->target.globe.sort_by_distance = (atoi(temp_input) == 1);
            break;

        case 3:
            query->type = SEARCH_TYPE_ASTRONOMICAL;
            if (!get_input("Enter Declination: ", temp_input, sizeof(temp_input))) return false;
            query->target.astronomical.dec = atof(temp_input);

            if (!get_input("Enter Right Ascension: ", temp_input, sizeof(temp_input))) return false;
            query->target.astronomical.ra = atof(temp_input);

            if (!get_input("Enter search radius (degrees): ", temp_input, sizeof(temp_input))) return false;
            query->target.astronomical.search_radius_degrees = (float)atof(temp_input);

            if (!get_input("Sort by distance? (1 = Yes [Top-K], 0 = No [Fast Stream]): ", temp_input, sizeof(temp_input))) return false;
            query->target.astronomical.sort_by_distance = (atoi(temp_input) == 1); 
            break;

        case 4:
            query->type = SEARCH_TYPE_TEMPORAL;
            if (!get_input("Enter Date Code (e.g., 19690720 for +1969-07-20): ", temp_input, sizeof(temp_input))) return false;

            int64_t date_code = 0;
            if (sscanf(temp_input, "%" SCNd64, &date_code) != 1) {
                printf("Invalid input! Please enter a valid integer date code.\n");
                return build_query(query, out_max_results);            
            }
            query->target.temporal.date_code = date_code;

            if (!get_input("Search forward in time? (1 = Yes, 0 = No [Backwards]): ", temp_input, sizeof(temp_input))) return false;
            query->target.temporal.search_forward = (atoi(temp_input) == 1);
            break;

        case 5:
            query->type = SEARCH_TYPE_QID;
            if (!get_input("Enter target QID (e.g., 42): ", temp_input, sizeof(temp_input))) return false;
            
            uint64_t qid = 0;
            if (sscanf(temp_input, "%" SCNu64, &qid) != 1) {
                printf("Invalid input! Please enter a valid QID.\n");
                return build_query(query, out_max_results);
            }
            query->target.qid.id = qid;

            if (!get_input("Search forward? (1 = Yes, 0 = No [Backwards]): ", temp_input, sizeof(temp_input))) return false;
            query->target.qid.search_forward = (atoi(temp_input) == 1);

            if (!get_input("Must match exactly? (1 = Yes [Strict], 0 = No [Paging]): ", temp_input, sizeof(temp_input))) return false;
            query->target.qid.first_result_must_match = (atoi(temp_input) == 1);
            break;

        default:
            printf("Invalid choice.\n");
            return build_query(query, out_max_results); 
    }

    if (!get_input("Enter maximum results to show (1-50): ", temp_input, sizeof(temp_input))) return false;

    int max = atoi(temp_input);
    if (max < 1) max = 1;
    if (max > MAX_DISPLAY_RESULTS) max = MAX_DISPLAY_RESULTS;

    *out_max_results = max;

    if (query->type == SEARCH_TYPE_GLOBE_COORDINATE) {
        query->target.globe.max_results = max;
    } else if (query->type == SEARCH_TYPE_ASTRONOMICAL) {
        query->target.astronomical.max_results = max;
    }

    return true;
}

int execute_and_display_search(DatabaseContext* ctx, SearchQuery* query, SearchResult* displayed_results, int max_matches) {
    SearchCursor* cursor = search_begin(ctx, query);
    if (cursor == NULL) {
        printf("Search initialization failed or index empty.\n");
        return 0;
    }

    SearchResult result;
    int match_count = 0;

    printf("\n--- SEARCH RESULTS ---\n");

    while (search_next(cursor, &result)) {
        if (match_count < max_matches) {
            displayed_results[match_count] = result;
        }
        match_count++;

        printf("[%d] Title: %s\n", match_count, result.title);
        printf("    Match: %s\n", result.term); 
        printf("    QID: Q%u | Length: %u bytes\n", result.qid, result.data_length);
        printf("------------------------\n");

        if (match_count >= max_matches) break;
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
        int requested_max_results = 0;

        if (!build_query(&query, &requested_max_results)) {
            break; 
        }

        SearchResult displayed_results[MAX_DISPLAY_RESULTS];

        int match_count = execute_and_display_search(ctx, &query, displayed_results, requested_max_results);

        if (match_count == 0) continue;

        while (true) {
            char choice_str[16];
            printf("\nEnter result number to read (1-%d), or 0 for new search: ", match_count);

            if (!get_input("", choice_str, sizeof(choice_str))) break;
            if (strlen(choice_str) == 0) continue;

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

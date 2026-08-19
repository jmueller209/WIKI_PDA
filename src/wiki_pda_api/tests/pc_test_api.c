#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <stdint.h>
#include "../include/wiki_pda.h"

int main(int argc, char** argv) {
    printf("--- ESP32 Memory-Constrained Simulation ---\n");

    FILE* db_file = fopen("bin/data_base.bin", "rb");
    if (!db_file) {
        printf("ERROR: Could not open bin/data_base.bin\n");
        return 1;
    }
    DatabasePlatform pc_platform = platform_desktop(db_file);

    DatabaseContext* ctx = db_init(INDEX_OMNI, pc_platform);
    if (ctx == NULL) {
        printf("ERROR: db_init failed.\n");
        fclose(db_file);
        return 1;
    }

    char input_buffer[256];

    while (true) {
        printf("\n========================================\n");
        printf("Enter search term (or 'quit'): ");

        if (fgets(input_buffer, sizeof(input_buffer), stdin) == NULL) {
            break;
        }

        input_buffer[strcspn(input_buffer, "\r\n")] = 0;

        if (strcmp(input_buffer, "quit") == 0 || strcmp(input_buffer, "exit") == 0) {
            break;
        }

        if (strlen(input_buffer) == 0) {
            continue;
        }

        SearchQuery query = {0};
        query.type = SEARCH_TYPE_OMNI;
        query.target.term = input_buffer; 
        query.article_type = 1;

        SearchCursor* cursor = search_begin(ctx, &query);
        if (cursor == NULL) {
            printf("No results found for '%s'.\n", input_buffer);
            continue;
        }

        SearchResult result;
        int match_count = 0;
        SearchResult displayed_results[10]; 

        printf("\n--- RESULTS FOR '%s' ---\n", input_buffer);

        while (search_next(cursor, &result)) {
            if (match_count < 10) {
                displayed_results[match_count] = result;
            }
            match_count++;

            printf("[%d] Match: %.*s\n", match_count, OMNI_SEARCH_TERM_SIZE, result.title);
            printf("    QID: Q%u | Length: %u bytes\n", result.qid, result.data_length);
            printf("------------------------\n");

            if (match_count >= 10) {
                break;
            }
        }

        search_end(cursor);

        if (match_count == 0) {
            printf("No matches found.\n");
            continue;
        } 

        while (true) {
            printf("\nEnter result number to read (1-%d), or 0 for new search: ", match_count);
            char choice_str[16];
            if (fgets(choice_str, sizeof(choice_str), stdin) == NULL) break;
            
            choice_str[strcspn(choice_str, "\r\n")] = 0;
            if (strlen(choice_str) == 0) {
                continue;
            }

            int choice = atoi(choice_str);
            if (choice == 0) {
                break;
            }
            
            if (choice > 0 && choice <= match_count) {
                SearchResult* selected = &displayed_results[choice - 1];
                if (selected->data_length == 0) {
                    printf("Empty article.\n");
                    continue;
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
                        if (!data_stream_read(stream, dummy_drain, chunk_target, &temp_read) || temp_read == 0) {
                            break;
                        }
                        bytes_skipped += temp_read;
                    }

                    uint32_t total_read_for_window = 0;
                    while (total_read_for_window < VIEW_WINDOW_SIZE) {
                        uint32_t to_read = VIEW_WINDOW_SIZE - total_read_for_window;
                        uint32_t chunk_bytes = 0;
                        if (!data_stream_read(stream, esp32_screen_buffer + total_read_for_window, to_read, &chunk_bytes) || chunk_bytes == 0) {
                            break;
                        }
                        total_read_for_window += chunk_bytes;
                    }

                    esp32_screen_buffer[total_read_for_window] = '\0';
                    data_stream_end(stream);

                    printf("\033[H\033[J");

                    printf("=== READING Q%u ===\n\n", selected->qid);
                    
                    if (total_read_for_window > 0) {
                        printf("%s\n", esp32_screen_buffer);
                    } else {
                        printf("[End of article reached]\n");
                    }
                    
                    printf("\n==========================================================\n");
                    printf("[Enter]: Scroll down | [p]: Scroll up | [q]: Exit reader\n");
                    printf("Command: ");

                    char cmd_buf[16];
                    if (fgets(cmd_buf, sizeof(cmd_buf), stdin) == NULL) break;
                    
                    cmd_buf[strcspn(cmd_buf, "\r\n")] = 0;

                    if (strcmp(cmd_buf, "q") == 0 || strcmp(cmd_buf, "Q") == 0) {
                        break;
                    } else if (strcmp(cmd_buf, "p") == 0 || strcmp(cmd_buf, "P") == 0) {
                        if (current_scroll_offset >= 256) {
                            current_scroll_offset -= 256;
                        } else {
                            current_scroll_offset = 0;
                        }
                    } else {
                        if (total_read_for_window > 0) {
                            current_scroll_offset += 256;
                        }
                    }
                }
            } else {
                printf("Invalid choice.\n");
            }
        }
    }

    db_end(ctx);
    fclose(db_file);
    return 0;
}

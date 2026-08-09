# C Query API Documentation

The C Query API is designed to be ultra-lightweight, memory-efficient, and platform-agnostic. It allows you to search the database and stream decompressed Wikipedia articles using only a few kilobytes of RAM.

## 1. Including the API in your project

To use the API, simply include the main header in your C/C++ files and link the compiled static/shared library (and ZSTD) during compilation.

```c
#include "wiki_db_api.h"
```

**Compiler Flags (Not tested):**
```bash
gcc main.c -o my_app -I./include -L./target -lwiki_query_api -lzstd
```

---

## 2. API Usage Walkthrough

Using the API generally follows a 4-step pipeline:
1. Initialize the Platform and Database Context.
2. Execute a Search.
3. Stream the Article Data.
4. Clean up.

### Step 1: Initialization (`DatabasePlatform` & `DatabaseContext`)

Because this engine is designed for embedded systems (like the ESP32), it does not assume you have a standard POSIX filesystem. All disk reads are abstracted through a `DatabasePlatform` struct. 

For standard PC testing, the API provides a built-in PC file wrapper.

```c
// Open the file using standard C I/O (PC only)
FILE* db_file = fopen("data_base.bin", "rb");
if (!db_file) return -1;

// Bind the file to the Platform interface
DatabasePlatform pc_platform = platform_pc_file(db_file);

// Initialize the Database Context
// Specify which indexes to load into RAM. Omni is the primary text search.
// You can load multiple indexes at once using db_init(INDEX_OMNI | OTHER_INDEX | .. , pc_platform);
// Currently available indexes: INDEX_OMNI
DatabaseContext* ctx = db_init(INDEX_OMNI, pc_platform);
if (ctx == NULL) {
    printf("Failed to initialize database!\n");
    fclose(db_file);
    return -1;
}
```

### Step 2: Searching (`SearchQuery` & `SearchCursor`)

To find an article, you populate a `SearchQuery` struct and pass it to `search_begin`. This returns a `SearchCursor` which you can use to iterate over the results.

```c
// Setup the query
SearchQuery query = {0};                 // Initialize everything to zero/NULL
query.type = SEARCH_TYPE_OMNI;           // Standard text-based search
query.target.term = "universe";          // The search term
query.article_type = 1;                  // 0 = metadata, check docs for other than 0

// Note: You can also set tags here to filter results. Just pass the bit mask as a uint32_t:
// query.exact_tags = ...
// query.include_tags = ...
// query.exclude_tags = ...

// Begin the search
SearchCursor* cursor = search_begin(ctx, &query);
if (cursor == NULL) {
    printf("No results found.\n");
    // Handle no results...
}

// Iterate over the results
SearchResult result;
while (search_next(cursor, &result)) {
    printf("Found: %s (QID: Q%u)\n", result.title, result.qid);
    printf("Data Offset: %llu, Compressed Size: %u\n", 
           result.data_offset, result.data_length);
    
    // For this example, we will just break after the first match
    break; 
}

// Free the cursor when done (important to avoid memory leaks)
search_end(cursor);
```

### Step 3: Reading an Article (`DataStream`)

Because articles can be huge and memory is limited, you might not want to load an entire article into RAM at once. Instead, you initialize a `DataStream`. The stream handles the ZSTD decompression dynamically as you request chunks of bytes.

```c
// Ensure we actually have an article with data
if (result.data_length > 0) {
    
    // Open the decompression stream pointing to the article's location
    DataStream* stream = data_stream_begin(ctx, result.data_offset, result.data_length);
    if (stream != NULL) {
        
        char text_buffer[512]; // Small RAM buffer!
        uint32_t bytes_read = 0;

        // Stream the article in 512-byte chunks
        while (data_stream_read(stream, text_buffer, sizeof(text_buffer) - 1, &bytes_read)) {
            if (bytes_read == 0) break; // End of article
            
            // Null-terminate and print to screen
            text_buffer[bytes_read] = '\0';
            printf("%s", text_buffer);
        }

        // Close the stream (Important to avoid memory leaks)
        data_stream_end(stream);
    }
}
```

### Step 4: Cleanup

When your application shuts down free the database context to prevent memory leaks.

```c
db_end(ctx);
fclose(db_file); // Don't forget to close your file handle
```

---

## 3. Porting to Embedded Hardware (e.g., ESP32)

To run this API on an ESP32 or custom hardware, you do **not** use `platform_pc_file()`. Instead, you define your own `DatabasePlatform` struct by writing a single function that tells the engine how to read raw bytes from your specific storage medium (like an SD card via SPI).

Here is a conceptual example:

```c
#include "database_platform.h"

// Write a custom read function for your hardware (must have this specific header)
bool platform_specific_read(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes, void* user_data) {
    // Implement Custom read logic here.
    // Cou can use the user_data to pass file handles for example
    return was_success;
}

// Inside your main function:
void app_main() {
    // Say your custom function needs a file handle like this one:
    FILE* f = fopen("/sdcard/data_base.bin", "rb");
    
    // Bind your custom function to the platform interface
    DatabasePlatform custom_platform;
    esp_platform.read_fn = platform_specific_read;
    esp_platform.user_data = f; // Pass the file pointer as the context
    
    // Initialize the DB exactly as normal.
    DatabaseContext* ctx = db_init(INDEX_OMNI, esp_platform);
    
    // ... run queries ...
}
```

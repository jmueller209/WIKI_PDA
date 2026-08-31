# C Query API Documentation

The C Query API is designed to allow you to query database created with the generator this repository provides. It aims to be ultra-lightweight, memory-efficient, and platform-agnostic. In this part of the documentation, the different functions and data structures of the API are explained as you'll walk through a simple step by step manual of how to setup a new PlatformIO project for a microcontroller and access the database. You can follow along step by step or only apply the core concepts to your own existing projects. For the step by step guide of creating a new PlatformIO project, etc. this guide assumes, that you have already flashed a database to an SD-card using the flashing tool provided by this repository. 

## 1. Setting up the project

As the main target of this project are microcontrollers like the Teensy or ESP32 that use PlatformIO for the setup and compilation process, this manual will also use PlatformIO. The core API functions and data structure are of course platform independent and can be used anywhere. While you can of course use editors like VSCode and their PlatformIO extensions, we will use the terminal and an editor of your choice in this guide. 

Open a new terminal window and install the PlatformIO Cli using the following command (Depending on your platform you might need to use `pip3` instead of `pip`):
```
pip install platformio
```
Create a new project directory, for example like this:
```
mkdir wiki_api_example
cd wiki_api_example
```
Now, initialize the project using PlatformIO. You'll need to specify the board you are using. In this guide I fill focus on the ESP32 and the Teensy 4.1 (Note that I am initializing the projects for both boards here. You can remove the `--board xyz` you do not need or use a different one):
```
pio project init --board esp32dev --board teensy41
```
Your project directory should now contain the following directories and files:
```
ls
include  lib  platformio.ini  src  test
```
If you are working with an ESP32 you will need to add the `SdFat` dependency to the `platformio.ini` file as it is not included by default:
```
[env:esp32dev]
platform = espressif32
board = esp32dev
framework = arduino
lib_deps =
    greiman/SdFat
```
Note: For the Teensy 4.1 you will not need to add any dependencies.
You now need to copy the `wiki_pda_api` into your project's `lib` directory. Assuming The cloned version of this repository is in the same directory as your PlatformIO project you can copy the necessary files like this:
```
cp -r ../WIKI_PDA/src/wiki_pda_api/ lib/
```
Now create your main file in the `src/` directory:
```
touch src/main.cpp
```
Open the file with an editor of your choice and paste the following code (We will only use this minimal initialization code to check if everything works correctly. In the next chapter we will actually go through what each line does):
```cpp
#include <Arduino.h>
#include "SdFat.h"
#include <wiki_pda.h>

SdFat sd;
DatabaseContext* ctx = nullptr;

void setup() {
    Serial.begin(115200);
    while (!Serial) { ; }

    Serial.println("Initializing SD card hardware...");

    #if defined(CORE_TEENSY)
        if (!sd.begin(BUILTIN_SDCARD)) {
            Serial.println("SD card initialization failed on Teensy!");
            return;
        }
    #elif defined(ESP32)
        const int chipSelect = 5; 
        if (!sd.begin(chipSelect, SPI_FULL_SPEED)) {
            Serial.println("SD card initialization failed on ESP32!");
            return;
        }
    #endif

    Serial.println("SD Card initialized. Mounting raw partition...");

    DatabasePlatform platform;

    #if defined(ESP32)
        Serial.println("Detected ESP32 Platform.");
        platform = platform_esp32((void*)sd.card());
    #elif defined(CORE_TEENSY)
        Serial.println("Detected Teensy Platform.");
        platform = platform_teensy((void*)sd.card());
    #else
        #error "Unsupported platform!"
    #endif

    ctx = db_init(INDEX_OMNI, platform);

    if (ctx != nullptr) {
        Serial.println("Wiki PDA initialized successfully from RAW partition!");
    } else {
        Serial.println("Database init failed! Check MBR, Magic String or Memory.");
    }
}

void loop() {
    delay(10000);
}
```
From the root directory of your project you can now run the following command to compile the code and check if the library integration works correctly:
```
pio run
```
To upload the compiled binary to your board, make sure the Teensy 4.1 or ESP32 is connected to your computer and run:
```
pio run -e esp32dev -t upload -t monitor
```
for the ESP32 or
```
pio run -e teensy41 -t upload -t monitor
```
for the Teensy 4.1. This will also open the serial monitor. You should see the following output:
```
Initializing SD card hardware...
SD Card initialized. Mounting raw partition...
Detected Teensy/ESP32 Platform.
Wiki PDA initialized successfully from RAW partition!
```
SD card not detected?
- Have you used the flashing tool provided by this repository to format the SD card and flash a valid database to it?
- Is the `const int chipSelect = 5;` correct (Only on ESP32) or do are you using a different IO-pin?

---

## 2. API Usage 

Using the API generally follows a 4-step pipeline:
1. Initialize the Platform and Database Context.
2. Execute a Search.
3. Stream the Article Data.
4. Clean up.

Now that the simple initialization program works, you can delete everything inside the `main.cpp` as we will now write the code step by step until you have a functioning program that allows you to read from the database.

### Step 1: API Initialization and Setup
Include the necessary headers:
```cpp
#include <Arduino.h>
#include "SdFat.h"
#include <wiki_pda.h>
```
We are using SdFat as it allows us hardware level access to the SD card without the filesystem abstraction layer. This is solely done to speed up database reads. We now define global constants to hold the handle to the SD card and a pointer the `DatabaseContext` struct that will be used internally by the API:
```
SdFat sd;
DatabaseContext* ctx = nullptr;
```
We can now write the `setup()` function:
```cpp
void setup() {
    // Initialize the serial interface
    Serial.begin(115200);
    while (!Serial) { ; }

    Serial.println("Initializing SD card hardware...");

    // Initialize the SD card based on your platform.
    // You can remove the preprocessor macros and only use
    // the code for the platform you are using.

    #if defined(CORE_TEENSY)
        if (!sd.begin(BUILTIN_SDCARD)) {
            Serial.println("SD card initialization failed on Teensy!");
            return;
        }
    #elif defined(ESP32)
        // You might need to change this to the specific chipSelect IO-pin
        // your card reader uses.
        const int chipSelect = 5;
        if (!sd.begin(chipSelect, SPI_FULL_SPEED)) {
            Serial.println("SD card initialization failed on ESP32!");
            return;
        }
    #endif

    Serial.println("SD card hardware initialized...");

    // We need to create a database platform. You can again remove
    // the preprocessor macros if you want the code to only work
    // for one specific platform.

    DatabasePlatform platform;

    #if defined(ESP32)
        Serial.println("Detected ESP32 Platform.");
        platform = platform_esp32((void*)sd.card());
    #elif defined(CORE_TEENSY)
        Serial.println("Detected Teensy Platform.");
        platform = platform_teensy((void*)sd.card());
    #else
        #error "Unsupported platform!"
    #endif

    // We are now initializing the database context

    ctx = db_init(INDEX_OMNI, platform);

    if (ctx != nullptr) {
        Serial.println("Wiki PDA initialized successfully from RAW partition!");
    } else {
        Serial.println("Database init failed! Check MBR, Magic String or Memory.");
    }
}
```
There are a few additional things to note here: The `platform_esp32` and `platform_teensy` functions are provided by the API. If you want to use the API on another platform, you will have to write the platform specific code yourself (See '[Defining your own database Platforms](Defining your own database Platforms)'). Moreover, you might want to take a closer look at the `db_init` function. The first parameter contains the indexes you want to load into RAM. The following indexes are available:
- `INDEX_OMNI`
- `INDEX_ASTRONOMICAL`
- `INDEX_TEMPORAL`
- `INDEX_GLOBE_COORDINATE`
The second parameter is the platform we defined above. You can load more than one Index at a time by chaining multiple indexes together using the pipe `|` symbol, e.g.:
```cpp
ctx = db_init(INDEX_OMNI | INDEX_TEMPORAL | INDEX_ASTRONOMICAL, platform);
```
If you want to use multiple Indexes in your program and you have enough RAM available you can always initialize all the indexes you need at the beginning. If you are really constrained in terms of RAM, you can always only use one Index at a time, free the context once you are done using it and create a new context with another Index later. For freeing resources, see '[Freeing API Ressources](Freeing API Ressources)'. 
The `db_init` function returns a pointer to the internally used `DatabaseContext` struct. For safety, I highly recommend checking if the returned pointer is not a `nullptr` to ensure that the initialization was successful.

### Step 2: Creating a Query
Note: The code in this and the following chapters should be written inside the `void loop(){...}` section of your program.
You can create an empty search query like this:
```cpp
// Create the query
SearchQuery query;

// Zero out the memory
memset(&query, 0, sizeof(SearchQuery));
```
The query struct contains a lot of fields of which some must not be populated. If you are not planning to populate all fields to have some sort of default behavior, you should always zero out the memory first. Even if you are planning to use every single field, I would still recommend zeroing out the memory in the beginning to prevent weird bugs that are difficult to debug. The first thing you will need to specify is the query type:
```cpp
query.type = SEARCH_TYPE_OMNI;
```
This specifies which index you want to query. The following search types are available:
- `SEARCH_TYPE_OMNI`
- `SEARCH_TYPE_TEMPORAL`
- `SEARCH_TYPE_GLOBE_COORDINATE`
- `SEARCH_TYPE_ASTRONOMICAL`
- `SEARCH_TYPE_QID`
- `SEARCH_TYPE_PID`

Note that the `SEARCH_TYPE_QID` and `SEARCH_TYPE_PID` will always work regardless of which indexes you have loaded in your `db_init` function. The other search types will only work if the corresponding index has been loaded into the database context. Depending on which search type you want to use you have different search options available:

**SEARCH_TYPE_OMNI**:
```cpp
// Specify the search term by which you want
// to search the omni search index. Any string
// is a valid search text.
query.target.omni.text = "uni";
```

**SEARCH_TYPE_TEMPORAL**:
```cpp
// The central target date formatted as (Year * 10000) + (Month * 100) + Day.
// Example: July 20, 1969 becomes 19690720. 
// Negative numbers represent BC dates (e.g., -5000101 for Jan 1st, 500 BC).
query.target.temporal.date_code = 19690720;

// Set to true to yielding results ranging into the future and to false 
// to yield results ranging into the past
query.target.temporal.search_forward = true;
```

**SEARCH_TYPE_GLOBE_COORDINATE**:
```cpp
// Target coordinates in decimal degrees (e.g., Eiffel Tower)
query.target.globe.lat = 48.8584; // [-90.0, 90.0]
query.target.globe.lon = 2.2945; // [-180.0, 180.0]

// Maximum search radius in kilometers
query.target.globe.search_radius_km = 50.0;

// Optional: Keep the closest results sorted by distance
// This will need additional computational resources and cache
// the results. Therefore, we also need to provide a
// 'max_results' parameter
query.target.globe.sort_by_distance = true;
query.target.globe.max_results = 10;
```

**SEARCH_TYPE_ASTRONOMICAL**:
```cpp
// Celestial coordinates in degrees (e.g., Sirius)
query.target.astronomical.dec = -16.7161; // Declination [-90.0 to +90.0]
query.target.astronomical.ra = 101.287;   // Right Ascension [0.0 to 360.0]

// Angular search radius in degrees
query.target.astronomical.search_radius_degrees = 5.0;

// Optional: Sort results from closest to furthest
// Same considerations as for the globe coordinate
// search apply here.
query.target.astronomical.sort_by_distance = true;
query.target.astronomical.max_results = 15;
```

**SEARCH_TYPE_QID** and **SEARCH_TYPE_PID**:
```cpp
// Exact Wikidata Item ID (QID) or Property ID (PID) to target
query.target.qid.id = 42; 

// True to search ascending IDs, false for descending
query.target.qid.search_forward = true;

// If true, the search fails if exactly Q42 doesn't exist. 
// If false, it acts like a pager and snaps to the nearest valid ID
// GREATER than the provided ID (if search_forward = true) or to the
// nearest valid ID LOWER than the provided ID (if search_forward = false)
query.target.qid.first_result_must_match = true; 
```

### Global Filters
Finally, no matter which search type you choose, you can apply **Global Filters** to narrow down your results. Because we zeroed out the memory initially, these will safely default to `0` (ignored) unless you explicitly set them:

```cpp
// Specify an exact tag mask that the results need to have to not be ignored.
// (Setting this to 0 will ignore this settings)
query.exact_tags = 0;

// Only return items that contain ALL of these tags
query.include_tags = 0;

// Reject items that contain ANY of these tags
query.exclude_tags = 0;

// Specify the article type to search for (metadata or article in specific language)
query.article_type = 0;
```
Note that all of the tags are of type `uint32_t` which represents a tag mask according to the tags you have 
specified in the `config.toml` file of your database generator. Setting the `article_type` to 0 will always
search for metadata, integers greater than zero refer to the actual text in a given language. To check which integer refers to which language, take a look at the `tmp/wiki_lang_mapping.txt` created by the database generator.
Here is another example of how to create a very simple search query:
```cpp
// Create query
SearchQuery query;
// Set default values
memset(query, 0, sizeof(SearchQuery));

// Search the omni search index
query.type = SEARCH_TYPE_OMNI;
// Search term "uni"
query.omni.text = "uni";
// Search for articles written in first language in mapping
query.article_type = 1;
```
**Important Note for PID Searches:**
PID Searches do not support `query.article_type = 1;` no metadata about the properties is saved except their name and description.

### Step 3: Executing a Query
Once you have created the search query you can perform the actual search like this:
```cpp
// begin the search
SearchCursor* cursor = search_begin(ctx, &query);

// Make sure no error occurred.
if (cursor == nullptr) {
    Serial.println("Search initialization failed or index empty.\n");
    return;
}
```
The `search_begin` function takes two parameters: The Database context and a reference to the search query. It returns a pointer to the internally used `SearchCursor` struct that functions as an iterator over the search results. You should again make sure that no `nullptr` is returned. This happens for example if you try to perform a search on an index that you have not loaded. You can now create an empty search result like this:
```cpp
SearchResult result;
```
To populate the result, use the `search_next` function:
```cpp
// perform the search and check if results have been found
bool success = search_next(cursor, &result);
if (success == false) {
    Serial.println("End of results reached");
}
search_end(cursor);
```
The function takes two parameters: A reference to the search cursor and a reference to the result. It returns a single boolean value telling you whether a result has been found or not. This is especially useful because you can call `search_next` multiple times (for example, in a `while` loop) to iterate over all the results based on the arguments provided by the search query. When you are done searching, do not forget to call `search_end` to free the memory.


The populated result contains the following pieces of information:
```cpp
uint32_t id = result.id;                 // The exact QID/PID of the matched item.
const char* title = result.title;        // A temporary text buffer containing the article title.
const char* term = result.term;          // A temporary text buffer containing the exact matched term, coordinate, or date.
uint32_t tags = result.tags;             // The tags assigned to this specific item.
uint32_t type = result.article_type;     // The type of payload available to fetch (Same as the one provided in the search query).
uint64_t offset = result.data_offset;    // The byte position in the database file where this article's data starts.
uint32_t length = result.data_length;    // The size of the compressed article (or metadata) data in bytes.
```

**Important Note for PID Searches:**
If you specified `SEARCH_TYPE_PID` in your query, the fields in the result must be interpreted slightly differently, as Wikidata properties do not have standalone Wikipedia articles:
- `result.title` will contain the **property title** (not an article title) in the search language.
- `result.term` will contain the **property description** (not the matched search term) in the search language.
- `result.data_offset`, `result.data_length`, `result.tags`,  are **not populated** and should be ignored.

### Step 4: Streaming Article Data and Metadata
Since individual articles can be too large to fit into RAM, it is often preferred to stream only parts of an article into RAM at once. To initiate an article stream you can use the following function:
```cpp
// Initialize data stream and check if the result is valid
DataStream* stream = data_stream_begin(ctx, result->data_offset, result->data_length);
if (stream == nullptr) {
    printf("Failed to open stream.\n");
    break;
}
```
The `data_stream_begin` function takes three arguments: The database context, the data offset which specifies the start of the data we want to read and the data length which specifies, how much data we want to read with the initiated stream. Note, that the data length does not specify the amount of data that is loaded into RAM but is used internally, when the end of the data is reached to prevent accidental reads beyond the end of an article. The function returns a pointer to an internally used `DataStream` struct. Make sure the function does not return a `nullptr` as this indicates that the initialization failed. This happens for example when trying to access invalid memory. You can now read the actual data into a buffer using the `data_stream_read` function:
```cpp
// Create a buffer of chars with a fixed size
// and a variable to keep track of the bytes read
char buffer[1024 + 1];
uint32_t bytes_read = 0;

// Read the data into the buffer and update bytes_read
bool end_reached = data_stream_read(stream, &buffer, 1024, &bytes_read)

// Null terminate the string for printing
buffer[bytes_read] = '\0';

// Print the read data
Serial.println(buffer);

// Do not forget to close the stream
data_stream_end(stream);
```

The `data_stream_read` function takes three arguments: The data stream, a pointer to a buffer of chars the actual data should be streamed into, the number of bytes to read and a pointer to an unsigned 32 bit integer that will contain the actual number of bytes read. Usually this will just be the specified amount of bytes to read, unless you have reached the end of the stream and there are only a few bytes left to read. The reason we are making the buffer one byte larger than the number of bytes were are reading is that strings in C are null-terminated but the `data_stream_read` function does only return the bytes it finds in the database which usually do not end in a null character. Therefore, we manually have to add `'\0'` after the last valid read byte. This way we can just print the string as usual.  Do not forget to close the stream using the `data_stream_end` function.

### Step 5: Freeing API Resources
At the end of your program or when you are done querying the database you have to free its resources. You can do this using the `db_end` function:
```cpp
db_end(ctx);
```

## Defining your own database Platforms

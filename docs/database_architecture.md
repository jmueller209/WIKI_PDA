# Database Architecture
This document outlines the internal structure and binary layout of the generated Wiki database. The database is designed as a single, contiguous binary file optimized for low-memory environments. You can look at the diagram [here](../README.md) to get a high level overview. Throughout this documentation ⚙️ `some_setting` is used to indicate that something is configurable via the [config_file](../config/config.toml).

---

## Table of Contents

- [0. High-Level Binary Layout](#0-high-level-binary-layout)
- [1. Header Design Philosophy](#1-header)
- [2. Primary Search Indexes (Flat k-trees)](#2-primary-search-indexes-flat-k-trees)
  - [2.1 Universal Row Structure](#21-universal-row-structure)
  - [2.2 The Specific Indexes](#22-the-specific-indexes)
  - [2.3 Creation of tags based on configuration](#24-creation-of-tags-based-on-configuration)
  - [2.4 Flat k-tree Traversal Visualization](#25-flat-k-tree-traversal-visualization)
- [3. Entity Indexes](#3-entity-indexes)
  - [3.1 QID Search Index](#31-qid-search-index)
  - [3.2 PID Search Index](#32-pid-search-index)
- [4. Data Storage](#4-data-storage)
  - [4.1 Metadata](#41-metadata)
  - [4.2 Content](#42-content)
- [5. Compression](#5-compression)
  - [5.1 ZSTD Dictionary](#51-zstd-dictionary)

 ---

## 0. High-Level Binary Layout

The entire database is bundled into a single binary file. While the exact byte-offsets depend on the generated content, the general structural sequence is as follows (Order might change / might not be accurate)

1. **Header** (Not implemented right now)
2. **Compression Dictionary** (ZSTD)
3. **Primary Search Indexes** (Flat k-trees)
4. **Entity Indexes** (QID & PID: Hashmaps)
5. **Metadata Storage** (Uncompressed entries that differ in size)
6. **Compressed Content Payload** (Compressed entries that differ in size)

---

## 1. Header
No header is used right now as the exact offsets, etc. are merged into the final query API binary via an [automatically generated C header file](../src/query_database/src/common/generated_database_constants.h). I might change this in the future and actually include a header to make the database more flexible. The disadvantage with the automatically generated header file is of course that we need to recompile the API code every time we want to use a new database. However, the advantage is that we can make the data structures and the API code as efficient as possible.

## 2. Primary Search Indexes (Flat k-trees)

To allow extremely fast lookups with almost zero RAM usage, the primary search indexes (Omni, Temporal, Global, Astronomical) are structured as **Flat k-trees** (Sparse Indexes). 

Instead of traditional pointers, the index is built in distinct "Levels" (Level 0 being the actual data, Level 1 being an index of Level 0, Level 2 being an index of Level 1, etc.). The top level is small enough to load completely into the ESP32's RAM, and it acts as a funnel pointing down to chunks in the lower levels.

### 2.1 Universal Row Structure
A massive optimization in this architecture is that **every row in every level is the exact same size**, and that total row size is strictly padded to the next **power of two**. This allows the C API to calculate disk offsets using fast bitwise shifts instead of expensive multiplication.

The total size of a row is calculated as: `Total Row Size = NextPowerOf2(Term Bytes + 8)`

#### Level 0 Rows (The Leaf Nodes)
Level 0 contains the actual search targets. Every entry maps a search term to a specific QID and its associated Search Tags.

| Offset | Size (Bytes) | Type | Name | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0` | `N` | `u8[]` | **Search Term** | The encoded search term (text, timestamp, or coordinates). Right-padded with zeros. |
| `N` | `4` | `u32` | **Target QID** | The integer QID this term points to (Little Endian). |
| `N+4` | `4` | `u32` | **Search Tags** | 1-hot encoded bitmask for filtering (e.g., `is_human` = bit 0). (Little Endian). |

#### Level 1+ Rows (The Sparse/Internal Nodes)
Levels 1 and above act as the search tree. To maintain a constant row size, the 8 bytes normally used for the QID and Tags are repurposed.

| Offset | Size (Bytes) | Type | Name | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0` | `N` | `u8[]` | **Search Term** | The exact search term copied from the first row of the child chunk. |
| `N` | `4` | `u32` | **Child Row Index** | The row index in the *previous* level (Level N-1) where this chunk begins. (Little Endian). |
| `N+4` | `4` | `[0u8; 4]` | **Padding** | Explicit zeros. Replaces the Tags field to maintain constant row size across all levels. |

---

### 2.2 The Specific Indexes

While all indexes share the universal structure above, the data stored inside the `Search Term` field differs based on the index type. 

#### Omni Search Index
* **Purpose:** The primary text-based index for searching concepts by name/title (e.g., "Albert Einstein").
* **Term Size:** Configurable via ⚙️ `omni_search_index_term_encoding_bytes`. However, if you provide a number of bytes so that the entire size of a row is not a power of 2, the generator will round up to the next power of 2 so you actually get a larger Term Size unless your provided a Term Size is of the form `power_of_two - 8`.
* **Row Size:** N (Term) + 4 (QID) + 4 (Tags) = N + 8 Bytes $\rightarrow$ Rounded to next power of 2.
* **Tags**: Bitmask created based on ⚙️ `omni_search_index_tags`  
* **Search strategy**: Use UTF-8 encoded text as search term. Case sensitivity is determined by ⚙️ `omni_search_index_case_sensitive`.

#### Temporal Search Index
* **Purpose:** Time-based index for querying concepts by specific dates or timespans.
* **Term Size:** 4 Bytes (`u32`).
* **Row Size:** 4 (Term) + 4 (QID) + 4 (Tags) = 12 Bytes $\rightarrow$ Padded to **16 Bytes**.
* * **Tags**: Bitmask created based on ⚙️ `temporal_search_index_tags`  
* **Search strategy**: TODO

#### Global Search Index
* **Purpose:** Spatial index for querying locations based on standard Earth coordinates.
* **Term Size:** 4 Bytes (`u32` encoded Latitude/Longitude).
* **Row Size:** 4 (Term) + 4 (QID) + 4 (Tags) = 12 Bytes $\rightarrow$ Padded to **16 Bytes**.
* * **Tags**: Bitmask created based on ⚙️ `globe_coordinate_search_index_tags`  
* **Search strategy**: TODO

#### Astronomical Search Index
* **Purpose:** Celestial spatial index for querying stars, galaxies, and astronomical bodies.
* **Term Size:** 4 Bytes (`u32` encoded Right Ascension/Declination).
* **Row Size:** 4 (Term) + 4 (QID) + 4 (Tags) = 12 Bytes $\rightarrow$ Padded to **16 Bytes**.
* * **Tags**: Bitmask created based on ⚙️ `astronomical_search_index_tags  `  
* **Search strategy**: TODO

### 2.4 Creation of tags based on configuration
TODO

---

### 2.5 Flat k-tree Traversal Visualization

When searching for a term, the C API loads the highest level (Top Level) into RAM. It performs a binary search to find the correct chunk, reads that chunk's `Child Row Index`, and calculates the absolute disk offset for the level below. 

It repeats this process, streaming one chunk of memory at a time, until it hits Level 0.

```text
[Top Level: Level 2] (Loaded in RAM)
Row 0: "Aar..."  -> Points to Level 1, Row 0
Row 1: "Ban..."  -> Points to Level 1, Row 500
Row 2: "Cat..."  -> Points to Level 1, Row 1000
       |
       v (Disk Seek & Read)
       
[Internal: Level 1] 
Row 500: "Ban..." -> Points to Level 0, Row 250000
Row 501: "Bar..." -> Points to Level 0, Row 250100
...
Row 999: "Cas..." -> Points to Level 0, Row 499900
       |
       v (Disk Seek & Read)
       
[Leaf Nodes: Level 0]
Row 250100: "Barcelona" -> QID: 1492, Tags: 0b0001
Row 250101: "Barium"    -> QID: 1100, Tags: 0b1000
Row 250102: "Bark"      -> QID: 8080, Tags: 0b0000
```

---

## 3. Entity Indexes

Once a search index yields a match, it points to a specific entity identifier (QID or later LID for Wiktionary). The metadata used properties (PID) to save information about a given concept (QID). The following indexes map those identifiers to their physical data locations.

### 3.1 QID Search Index

The QID index connects a generic concept identifier (QID) and the actual storage layer. When a primary search index yields a target QID, the API uses this index to find all concrete instances of that concept across different languages and projects (e.g., English Wikipedia vs. German Wikipedia).

The QID search index consists of two distinct components:
1. **The Hash Map (Lookup Table):** Maps an integer QID to a range within the row table.
2. **The Index Row Table:** A contiguous array of records pointing to the actual metadata and content payloads on disk.

#### 1. Hash Map Table Entry (`HashMapRow`)
Every entry in this table corresponds to a QID, pointing the engine to where its specific instances begin.

| Offset | Size (Bytes) | Type | Name | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0` | `4` | `u32` | **Start Index** | The starting row index in the Index Row Table where this QID's entries begin. |
| `4` | `2` | `u16` | **Entry Count** | The number of consecutive rows belonging to this QID (e.g., how many languages/projects exist for it). |

#### 2. Index Row Table Entry (`IndexRow`)
Once the Hash Map yields a start index and count, the API reads these rows to find the physical storage locations. Because a single concept (QID) can exist in multiple languages or projects, a single QID can point to multiple `IndexRow` entries.

| Offset | Size (Bytes) | Type | Name | Description |
| :--- | :--- | :--- | :--- | :--- |
| `0` | `8` | `u64` | **Offset** | The byte offset of the data chunk on disk. |
| `8` | `4` | `u32` | **Length** | The length of the compressed payload in bytes. |
| `12` | `2` | `u16` | **Project ID** | Identifies the language and wiki project (e.g., enwiki, dewiki). |

- Note 1: Project ID 0 always refers to metadata. How the other IDs refer to different languages and concepts depends on your specific database setup. The generator will create a `wiki_lang_mapping.txt` file in the ⚙️ `tmp_dir`, where you can check which project maps to which ID. I might change this in the future and include the mapping directly in the database header for easier access.
- Note 2: The **Offset** is to be understood relative to the begin of the metadata (Project ID = 0) or Content (Project ID >= 1). To get the absolute offsets inside the binary, the API adds a global metadata/content offset which is specified in the [automatically generated C header file](../src/query_database/src/common/generated_database_constants.h).

### 3.2 PID Search Index
* **Purpose:** Stores property definitions (e.g., `P31` = "instance of"). Used to interpret and correctly render the compressed metadata.
* **Structure Details:** TODO

---

## 4. Data Storage

This section contains the actual payloads returned to the user after a successful search and routing.

### 4.1 Metadata
* **Purpose:** Stores the wiki properties (PIDs) and tags associated with an article (e.g., coordinate data, categorization flags).
* **Structure Details:** TODO

### 4.2 Content
* **Purpose:** The actual article text/HTML, stripped and formatted. 
* **Compression:** Stored in compressed chunks using ZSTD.
* **Structure Details:** TODO

---

## 5. Compression

### 5.1 ZSTD Dictionary
* **Purpose:** A pre-trained Z-Standard dictionary By training a dictionary on Wiki data during the generation phase, the engine can decompress tiny data chunks (like individual articles) with extremely high compression ratios.
* **Structure Details:** TODO

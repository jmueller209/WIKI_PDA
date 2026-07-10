# Cyberdeck Global Offline Database Architecture

## Executive Summary
This document outlines the file structures and C++ lookup strategies for the ESP32 offline Wikipedia reader. The architecture completely separates the **Search Index** (how users find concepts) from the **Data Heap** (the actual text), allowing infinite language scaling, lightning-fast searches, and FAT32 compatibility, all while using less than 2 KB of RAM.

---

## 1. The Omni-Search Index (`global_search.bin`)
**Purpose:** The single entry point for user text input. Contains every article title, alias, and translation for all downloaded languages, merged into one file.
**Sorting:** Strictly Alphabetical (UTF-8 byte values).
**Execution:** ESP32 uses `strncmp()` binary search with a "rewind and read forward" algorithm to capture autocompletes and duplicate terms (homonyms).

### Binary Layout (72 Bytes / Record Fixed-Width)

| Field Name | Type | Size | Python Struct | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Term** | `char[64]` | 64 Bytes | `64s` | Null-padded UTF-8 search string |
| **Q-ID** | `uint32_t` | 4 Bytes | `I` | Universal Concept ID |
| **Lang ID**| `uint16_t` | 2 Bytes | `H` | E.g., `0`=EN, `1`=DE, `2`=JA |
| **Type** | `uint16_t` | 2 Bytes | `H` | `0`=Direct Article, `1`=Alias/Term |

### C++ Struct Definition
```cpp
struct __attribute__((__packed__)) GlobalSearchEntry {
    char term[64];
    uint32_t q_id;
    uint16_t lang_id;
    uint16_t type_flag;
};
```

---

## 2. The Vertical Master Map (`master_index.bin`)
**Purpose:** Connects a Q-ID to the physical SD card byte offset of the actual text. 
**Sorting:** Strictly Numerical (Lowest Q-ID to Highest).
**Scaling Strategy:** "Vertical Indexing." If an article exists in 5 languages, it has 5 tiny rows grouped together. If it exists in 1 language, it has 1 row. Zero wasted space.

### The Bit-Packed Chunk ID
To bypass the 4 GB file size limit of FAT32 and the 4 GB integer limit of `uint32_t`, the 16-bit Language ID field is split using bitwise logic:
* **Lowest 10 Bits:** Language Code (Supports up to 1,024 languages).
* **Highest 6 Bits:** File Chunk ID (Supports up to 64 file chunks per language, e.g., `en_0.bin`, `en_1.bin`).

### Binary Layout (10 Bytes / Record Fixed-Width)

| Field Name | Type | Size | Python Struct | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Q-ID** | `uint32_t` | 4 Bytes | `I` | Universal Concept ID |
| **Lang/Chunk** | `uint16_t` | 2 Bytes | `H` | Bit-packed Language and Chunk ID |
| **Offset** | `uint32_t` | 4 Bytes | `I` | Byte offset inside the target chunk file |

### C++ Struct Definition
```cpp
struct __attribute__((__packed__)) MasterMapEntry {
    uint32_t q_id;
    uint16_t packed_lang_chunk;
    uint32_t file_offset;
};
```

---

## 3. The Data Heap Chunks (`/data/lang_chunk.bin`)
**Purpose:** Massive, unsorted storage blobs containing the actual article text.
**Naming Convention:** `en_0.bin`, `en_1.bin`, `de_0.bin`. No file ever exceeds 3.9 GB to maintain universal FAT32 compatibility.
**Execution:** The ESP32 never searches these files. It uses `File.seek(offset)` to jump instantly to the payload.

### The Length-Prefixed Architecture (Pascal Strings)
To allow modular OTA updates and avoid complex "next offset" math across different language chunks, every article is prefixed with its exact byte length.

| 4-Byte Prefix (`uint32_t`) | Variable Length Payload (Raw Text) |
| :--- | :--- |
| `18` | `Dies ist die Sonne` |
| `25` | `Bratwurst ist sehr lecker` |

### C++ Read Execution
```cpp
// After seeking to the offset provided by the Master Map:
uint32_t articleLength;
dataFile.read((uint8_t*)&articleLength, sizeof(uint32_t));

for (uint32_t i = 0; i < articleLength; i++) {
    char c = dataFile.read();
    printToScreen(c); 
}
```

---

## 4. System Requirements & Hardware Flow
1. **SD Card Formatting:** Force-format any high-capacity SD card (64GB - 1TB) to **FAT32** with a 32KB or 64KB cluster size for maximum read speeds.
2. **RAM Preservation:** The ESP32 relies heavily on `File.seek()`. No index file is ever loaded into RAM. Binary searches are executed by jumping the SD card read-head and loading a single fixed-width struct at a time (e.g., max 72 Bytes).

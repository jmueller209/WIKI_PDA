# Database Architecture

This document outlines the internal structure and binary layout of the generated Wiki database.

The database is designed as a single contiguous binary file optimized for efficient random access and low-memory environments. The database contains the indexes, titles, metadata, compressed article content, and the ZSTD compression dictionary required by the query API.

Throughout this documentation, ⚙️ `some_setting` is used to indicate that something is configurable via the [config_file](../config/config.toml).

---

## Table of Contents

- [0. High-Level Binary Layout](#0-high-level-binary-layout)
- [1. Database Header](#1-database-header)
  - [1.1 Header Structure](#11-header-structure)
  - [1.2 Section Offsets and Sizes](#12-section-offsets-and-sizes)
  - [1.3 Primary Search Index Metadata](#13-primary-search-index-metadata)
- [2. Primary Search Indexes](#2-primary-search-indexes)
  - [2.1 Universal Row Structure](#21-universal-row-structure)
  - [2.2 Level 0 Rows](#22-level-0-rows)
  - [2.3 Sparse Rows](#23-sparse-rows)
  - [2.4 The Specific Indexes](#24-the-specific-indexes)
  - [2.5 Flat k-tree Traversal](#25-flat-k-tree-traversal)
- [3. Entity Indexes](#3-entity-indexes)
  - [3.1 QID Search Index](#31-qid-search-index)
  - [3.2 PID Search Index](#32-pid-search-index)
- [4. Shared String Storage](#4-shared-string-storage)
  - [4.1 Article Titles](#41-article-titles)
  - [4.2 PID Strings](#42-pid-strings)
- [5. Data Storage](#5-data-storage)
  - [5.1 Content](#51-content)
  - [5.2 Metadata](#52-metadata)
- [6. Compression](#6-compression)
  - [6.1 ZSTD Dictionary](#61-zstd-dictionary)
- [7. Binary Data Types and Offset Rules](#7-binary-data-types-and-offset-rules)

---

## 0. High-Level Binary Layout

The entire database is bundled into a single binary file.

The database is written in the following order:

1.  Database Header
2.  Primary Search Indexes
    2.1 Omni Search Index
    2.2 Temporal Search Index
    2.3 Astronomical Search Index
    2.4 Globe Coordinate Search Index
3.  QID Hash Map
4.  QID Index
5.  Article Titles
6.  PID Hash Map
7.  PID Index
8.  PID Strings
9.  Compressed Article Content
10. Metadata
11. ZSTD Dictionary

The exact byte offsets and section sizes are stored directly in the database header. This means the query API does not need a separately generated header file containing database-specific offsets.

The header therefore provides everything required to locate the individual sections of the database.

---

# 1. Database Header

The database begins with a fixed-layout `DatabaseHeader`. The query API reads the header from byte offset `0`, validates its magic and version, and then uses its offsets and sizes to access the remaining sections.

Unlike the previous architecture, database-specific offsets are **stored inside the database itself** rather than being generated into a C header file.

## 1.1 Header Structure

The current header is:

```c
typedef struct {
    char magic[4];
    uint32_t version;

    uint64_t offset_qid_hashmap;
    uint64_t offset_qid_index;
    uint64_t offset_titles;
    uint64_t offset_pid_hashmap;
    uint64_t offset_pid_index;
    uint64_t offset_pid_strings;
    uint64_t offset_content;
    uint64_t offset_metadata;
    uint64_t offset_zstd_dictionary;

    uint64_t size_qid_hashmap;
    uint64_t size_qid_index;
    uint64_t size_titles;
    uint64_t size_pid_hashmap;
    uint64_t size_pid_index;
    uint64_t size_pid_strings;
    uint64_t size_content;
    uint64_t size_metadata;
    uint64_t size_zstd_dictionary;

    IndexMetadata omni_search;
    IndexMetadata temporal_search;
    IndexMetadata astro_search;
    IndexMetadata globe_search;
} DatabaseHeader;
```

The header contains three categories of information:

1. **Identification**
   - `magic`
   - `version`

2. **Global database sections**
   - one absolute offset and one size for each major section

3. **Primary search index metadata**
   - one `IndexMetadata` structure for each primary search index

### `IndexMetadata`

Each primary search index is described by:

```c
typedef struct {
    uint8_t is_enabled;
    uint8_t num_sparse_levels;
    uint16_t _padding1;

    uint32_t top_level_rows;
    uint32_t term_size;
    uint32_t row_size;
    uint32_t chunk_size;
    uint32_t _padding2;

    uint64_t level_offsets[MAX_SPARSE_LEVELS];
    uint64_t level_sizes[MAX_SPARSE_LEVELS];
} IndexMetadata;
```

The fields have the following meanings:

| Field | Description |
| :--- | :--- |
| `is_enabled` | Indicates whether this search index is present and usable. |
| `num_sparse_levels` | Number of sparse levels above Level 0. |
| `_padding1` | Explicit padding reserved for the binary layout. |
| `top_level_rows` | Number of rows in the top sparse level, which is loaded into RAM for searching. |
| `term_size` | Number of bytes occupied by the encoded search term. |
| `row_size` | Size of a row in the corresponding index representation. |
| `chunk_size` | Number of rows searched as one streamed chunk while traversing lower levels. |
| `_padding2` | Explicit padding/reserved space. |
| `level_offsets[]` | Absolute byte offsets of the individual index levels in the database. |
| `level_sizes[]` | Size in bytes of the individual index levels. |

`level_offsets[]` and `level_sizes[]` refer directly to positions in the complete database binary. They are therefore **absolute database offsets**, unlike the payload offsets stored in `QIDIndexRow`.

Level `0` is always the leaf/base level. Levels above it are sparse search levels.

---

## 1.2 Section Offsets and Sizes

The following header fields describe the major database sections:

| Section | Offset Field | Size Field | Meaning |
| :--- | :--- | :--- | :--- |
| QID Hash Map | `offset_qid_hashmap` | `size_qid_hashmap` | QID lookup table |
| QID Index | `offset_qid_index` | `size_qid_index` | Rows associated with QIDs |
| Titles | `offset_titles` | `size_titles` | Article title string storage |
| PID Hash Map | `offset_pid_hashmap` | `size_pid_hashmap` | PID lookup table |
| PID Index | `offset_pid_index` | `size_pid_index` | Language-specific property entries |
| PID Strings | `offset_pid_strings` | `size_pid_strings` | Shared PID title/description strings |
| Content | `offset_content` | `size_content` | Compressed article payloads |
| Metadata | `offset_metadata` | `size_metadata` | Metadata payloads |
| ZSTD Dictionary | `offset_zstd_dictionary` | `size_zstd_dictionary` | ZSTD dictionary bytes |

All `offset_*` fields are **absolute offsets from the beginning of the database file**.

All `size_*` fields are byte lengths of the corresponding section.

---

## 1.3 Primary Search Index Metadata

Four primary search indexes are described in the database header:

```text
omni_search
temporal_search
astro_search
globe_search
```

Each index can independently be enabled or disabled.

The metadata stored in the header allows the query API to determine:

- whether an index exists,
- how many sparse levels it contains,
- how many rows are in the top level,
- how large each row is,
- how large each streamed search chunk is,
- and where each level is physically located in the database.

This replaces the need for database-specific generated offset constants.

---

# 2. Primary Search Indexes

To allow extremely fast lookups with very little RAM usage, the primary search indexes are structured as **flat k-trees / sparse indexes**.

Instead of traditional pointers, the index is built in distinct levels:

- **Level 0** contains the actual searchable entries.
- **Level 1** is a sparse index of Level 0.
- **Level 2** is a sparse index of Level 1.
- and so on.

The top sparse level is small enough to load completely into RAM. It acts as a funnel that points the search towards the appropriate chunk in the next lower level.

The primary indexes are:

- Omni Search
- Temporal Search
- Astronomical Search
- Globe Coordinate Search

The exact row representation depends on the index type, but all of them use the same hierarchical traversal strategy.

---

## 2.1 Universal Row Structure

The primary search indexes distinguish between two types of rows:

1. **Base rows** at Level 0
2. **Sparse rows** at Levels 1 and above

The base rows contain the actual search target and the QID/tag information.

Sparse rows contain the search term and a row index pointing into the next lower level.

The binary representation of the rows is fixed-size for a given index. The corresponding row size is stored in `IndexMetadata.row_size`.

The primary numeric indexes use 16-byte rows. Omni rows use a configurable term size followed by the fixed QID/tag or sparse-row fields.

---

## 2.2 Level 0 Rows

Level 0 contains the actual search targets.

### Astronomical

```c
typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} AstronomicalRow;
```

| Offset | Size | Type | Name | Description |
| :--- | ---: | :--- | :--- | :--- |
| `0` | `8` | `int64_t` | `term` | Encoded astronomical search term. |
| `8` | `4` | `uint32_t` | `qid` | Target QID. |
| `12` | `4` | `uint32_t` | `tags` | Search-tag bitmask. |

Total size: **16 bytes**.

### Globe Coordinates

```c
typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} GlobeCoordinateRow;
```

The binary layout is identical to `AstronomicalRow`.

Total size: **16 bytes**.

### Temporal

```c
typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t qid;
    uint32_t tags;
} TemporalRow;
```

The binary layout is identical to `AstronomicalRow`.

Total size: **16 bytes**.

### Omni

```c
typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t qid;
    uint32_t tags;
} OmniRow;
```

| Offset | Size | Type | Name | Description |
| :--- | ---: | :--- | :--- | :--- |
| `0` | `OMNI_SEARCH_TERM_SIZE` | `char[]` | `term` | Fixed-size encoded UTF-8 search term. |
| `N` | `4` | `uint32_t` | `qid` | Target QID. |
| `N+4` | `4` | `uint32_t` | `tags` | Search-tag bitmask. |

where `N = OMNI_SEARCH_TERM_SIZE`.

Total size: **`OMNI_SEARCH_TERM_SIZE + 8` bytes**.

---

## 2.3 Sparse Rows

Sparse levels use rows with the same basic term size as their corresponding base index, followed by a row pointer into the next lower level.

### Numeric Sparse Rows

Astronomical, globe-coordinate, and temporal indexes use:

```c
typedef struct __attribute__((packed)) {
    int64_t term;
    uint32_t target_row;
    uint8_t _padding[4];
} AstronomicalSparseRow;
```

The globe and temporal sparse rows use the same layout.

| Offset | Size | Type | Name | Description |
| :--- | ---: | --- | :--- | :--- |
| `0` | `8` | `int64_t` | `term` | Search term identifying the beginning of the corresponding lower-level chunk. |
| `8` | `4` | `uint32_t` | `target_row` | Row index in the lower level where the selected chunk begins. |
| `12` | `4` | `uint8_t[4]` | `_padding` | Explicit padding/reserved bytes. |

Total size: **16 bytes**.

### Omni Sparse Rows

```c
typedef struct __attribute__((packed)) {
    char term[OMNI_SEARCH_TERM_SIZE];
    uint32_t target_row;
    uint8_t _padding[4];
} OmniSparseRow;
```

Total size: **`OMNI_SEARCH_TERM_SIZE + 8` bytes**.

The sparse rows do not store a QID or tag bitmask. The final QID and tags are obtained from the Level 0 row reached at the end of traversal.

---

## 2.4 The Specific Indexes

All four indexes use the sparse-level architecture described above, but the searchable term has a different meaning.

### Omni Search Index

**Purpose:** The primary text-based index for searching concepts by names and titles.

**Row type:** `OmniRow`

**Sparse row type:** `OmniSparseRow`

**Term size:** Configurable through ⚙️ `omni_search_index_term_encoding_bytes` and stored in `IndexMetadata.term_size`.

**Search strategy:** UTF-8 encoded text.

**Case sensitivity:** Determined by ⚙️ `omni_search_index_case_sensitive`.

**Tags:** The bitmask is created from ⚙️ `omni_search_index_tags`.

---

### Temporal Search Index

**Purpose:** Time-based lookup of concepts by dates or temporal values.

**Row type:** `TemporalRow`

**Sparse row type:** `TemporalSparseRow`

**Term size:** 8 bytes (`int64_t`).

**Row size:** 16 bytes.

**Tags:** The bitmask is created from ⚙️ `temporal_search_index_tags`.

**Search strategy:** The index is traversed using the numeric term ordering and the generic numeric k-tree search routine.

---

### Globe Coordinate Search Index

**Purpose:** Spatial lookup for locations represented using global Earth coordinates.

**Row type:** `GlobeCoordinateRow`

**Sparse row type:** `GlobeCoordinateSparseRow`

**Term size:** 8 bytes (`int64_t`).

**Row size:** 16 bytes.

**Tags:** The bitmask is created from ⚙️ `globe_coordinate_search_index_tags`.

**Search strategy:** The encoded numeric coordinate term is searched using the same sparse-level traversal mechanism as the other numeric indexes.

---

### Astronomical Search Index

**Purpose:** Spatial lookup for astronomical objects such as stars, galaxies, and other celestial bodies.

**Row type:** `AstronomicalRow`

**Sparse row type:** `AstronomicalSparseRow`

**Term size:** 8 bytes (`int64_t`).

**Row size:** 16 bytes.

**Tags:** The bitmask is created from ⚙️ `astronomical_search_index_tags`.

**Search strategy:** The encoded numeric astronomical term is searched using the same sparse-level traversal mechanism.

---

## 2.5 Flat k-tree Traversal

When searching for a term, the query API first loads the **top sparse level** into RAM.

The number of rows loaded is stored in:

```text
IndexMetadata.top_level_rows
```

The API performs a binary search over this in-memory level to select the appropriate child chunk.

The selected sparse row contains a `target_row`, which points to the beginning of a chunk in the next lower level.

The lower level is not loaded completely into RAM. Instead, the API reads only the required chunk and performs another binary search.

This process continues until Level 0 is reached.

Finally, the API searches the selected Level 0 chunk for the requested term.

Conceptually:

```text
[Top Level: Level N]  ← loaded into RAM
        |
        | target_row
        v
[Level N-1]            ← one chunk read from disk
        |
        | target_row
        v
[Level N-2]            ← one chunk read from disk
        |
        v
       ...
        |
        v
[Level 0]              ← one chunk read from disk
        |
        v
[QID + Tags]
```

For each level, the absolute address of a row is calculated from:

```text
level_offset + row_index × row_size
```

where `level_offset` comes from `IndexMetadata.level_offsets[]`.

The stored level size is used for bounds checking.

This structure provides logarithmic narrowing while keeping RAM usage low: only the top level and one temporary row buffer need to be held in memory during traversal.

---

# 3. Entity Indexes

Once a primary search index yields a QID, the query API uses the QID index to locate the corresponding article or metadata records.

The entity indexes are separate from the primary search indexes.

They are responsible for mapping entity identifiers to the physical payloads stored in the database.

---

## 3.1 QID Search Index

The QID search index consists of two contiguous structures:

1. **QID Hash Map**
2. **QID Index**

The hashmap maps a QID to a contiguous range of `QIDIndexRow` records.

A single QID may have several index rows because the same Wikidata entity can have an article in multiple languages and can also have a metadata representation.

### QID Hash Map

Each hashmap row is:

```c
typedef struct __attribute__((packed)) {
    uint32_t start_index;
    uint16_t entry_count;
} QIDHashMapRow;
```

| Offset | Size | Type | Name | Description |
| :--- | ---: | --- | :--- | :--- |
| `0` | `4` | `uint32_t` | `start_index` | Index of the first `QIDIndexRow` belonging to this QID. |
| `4` | `2` | `uint16_t` | `entry_count` | Number of consecutive `QIDIndexRow` records belonging to this QID. |

Total size: **6 bytes**.

The hashmap rows are stored in QID order. The position of a row therefore corresponds directly to the QID.

QIDs for which no valid entry exists may have an empty hashmap row. Such a row has no associated `QIDIndexRow` entries.

The hashmap does not itself contain the individual language/project records. Instead, `start_index` and `entry_count` identify a range in the separate QID Index.

### QID Index

Each QID index row is:

```c
typedef struct __attribute__((packed)) {
    uint64_t offset;
    uint32_t length;
    uint16_t project_id;
    uint32_t title_offset;
} QIDIndexRow;
```

| Offset | Size | Type | Name | Description |
| :--- | ---: | --- | :--- | :--- |
| `0` | `8` | `uint64_t` | `offset` | Relative offset of the payload within the relevant content or metadata section. |
| `8` | `4` | `uint32_t` | `length` | Compressed payload length in bytes. |
| `12` | `2` | `uint16_t` | `project_id` | Identifies the payload type/language. |
| `14` | `4` | `uint32_t` | `title_offset` | Offset into the article title section. |

Total size: **18 bytes**.

A single QID may therefore point to multiple records:

```text
QID
 └── QIDHashMapRow
      ├── start_index
      └── entry_count
             |
             v
      QIDIndexRow[]
        ├── project_id = 0  → metadata
        ├── project_id = 1  → article in one language/project
        ├── project_id = 2  → article in another language/project
        └── ...
```

### Project IDs

`project_id` determines what the QID index row refers to:

- **`project_id == 0`**: the row refers to metadata.
- **`project_id > 0`**: the row refers to an article for a specific language/project.

The language/project mapping for positive project IDs is generated as part of the database-generation process.

### Relative payload offsets

The `offset` field in `QIDIndexRow` is **not an absolute database offset**.

It is relative to the relevant database section.

For an article:

```text
absolute article offset =
    header.offset_content
    + QIDIndexRow.offset
```

For metadata:

```text
absolute metadata offset =
    header.offset_metadata
    + QIDIndexRow.offset
```

The section is determined by `project_id`.

The payload length stored in `length` is the **compressed length** of the payload.

Thus, the database header defines the physical location of each major section, while the QID index supplies the location inside that section.

---

## 3.2 PID Search Index

The PID index follows the same general two-stage design as the QID index:

1. **PID Hash Map**
2. **PID Index**

A PID may have entries for multiple languages. Each language-specific entry stores references to the property's title and description.

### PID Hash Map

```c
typedef struct __attribute__((packed)) {
    uint32_t start_index;
    uint16_t entry_count;
} PIDHashMapRow;
```

The layout is identical to `QIDHashMapRow`.

Each hashmap row identifies the contiguous range of `PIDIndexRow` entries belonging to one property.

The rows are stored in PID order.

---

### PID Index

```c
typedef struct __attribute__((packed)) {
    uint16_t project_id;
    uint32_t title_offset;
    uint32_t desc_offset;
} PIDIndexRow;
```

| Offset | Size | Type | Name | Description |
| :--- | ---: | --- | :--- | :--- |
| `0` | `2` | `uint16_t` | `project_id` | Language/project identifier. |
| `2` | `4` | `uint32_t` | `title_offset` | Offset into the PID string pool for the property title. |
| `6` | `4` | `uint32_t` | `desc_offset` | Offset into the PID string pool for the property description. |

Total size: **10 bytes**.

A single PID can therefore have multiple language-specific rows:

```text
PID
 └── PIDHashMapRow
      ├── start_index
      └── entry_count
             |
             v
      PIDIndexRow[]
        ├── language/project A
        ├── language/project B
        └── ...
```

`project_id` is always greater than zero for PID index records. PID records represent language-specific property information and do not use the metadata project (`0`).

The title and description themselves are not stored directly in the fixed-size index row because their lengths are variable. Instead, both strings are stored in the separate PID string pool, and the index contains offsets into that pool.

---

# 4. Shared String Storage

Variable-length strings are stored separately from fixed-size index rows.

This keeps the indexes compact and allows strings to be stored without reserving a fixed amount of space per record.

---

## 4.1 Article Titles

The `titles` section stores article titles separately from the QID index.

`QIDIndexRow.title_offset` is an offset into the title section:

```text
QIDIndexRow.title_offset
        |
        v
header.offset_titles
        +
title_offset
        |
        v
stored title
```

This avoids storing a variable-length title inside every fixed-size `QIDIndexRow`.

---

## 4.2 PID Strings

The PID string section contains the titles and descriptions of the Wikidata properties included in the database.

Strings are stored as UTF-8 bytes followed by a NUL terminator (`0x00`).

The string pool begins with a single zero byte. Consequently:

```text
offset 0 → empty / no string
```

New strings are deduplicated during generation: if the same string occurs more than once, the existing string-pool offset is reused.

Conceptually:

```text
PIDIndexRow
 ├── title_offset ──────┐
 └── desc_offset ───────┤
                        v
                 PID String Pool
                 ├── "instance of\0"
                 ├── "subclass of\0"
                 ├── ...
                 └── ...
```

The offsets stored in `PIDIndexRow` are relative to the PID string section.

The absolute database address of a string is therefore:

```text
absolute string address =
    header.offset_pid_strings
    + PIDIndexRow.title_offset
```

or:

```text
absolute string address =
    header.offset_pid_strings
    + PIDIndexRow.desc_offset
```

---

# 5. Data Storage

The database stores two main types of payload data:

- compressed article content
- metadata

The corresponding QID index records provide the relative payload offset and compressed length.

---

## 5.1 Content

The content section contains the processed article payloads.

Article content is stored in compressed form using ZSTD.

For an article referenced by a `QIDIndexRow` with `project_id > 0`:

```text
absolute article offset =
    header.offset_content
    + qid_index_row.offset
```

The number of bytes to read is:

```text
qid_index_row.length
```

Because the length stored in the index is the compressed length, the API reads the compressed payload and decompresses it using the database's ZSTD dictionary.

The content section is therefore independent of the absolute position of the database as a whole: the QID index only needs a compact section-relative offset.

---

## 5.2 Metadata

The metadata section contains the metadata representation associated with an entity.

Metadata is referenced using the same `QIDIndexRow` structure as article content.

For a metadata row:

```text
project_id == 0
```

and:

```text
absolute metadata offset =
    header.offset_metadata
    + qid_index_row.offset
```

The `length` field gives the compressed payload length stored at that relative offset.

This allows metadata and article content to share a common QID-to-payload indexing structure while remaining physically separated into their own database sections.

---

# 6. Compression

## 6.1 ZSTD Dictionary

The database contains a ZSTD dictionary trained on the data generated for the database.

The dictionary is stored in the dedicated:

```text
ZSTD Dictionary
```

section.

Its location and size are given by:

```text
header.offset_zstd_dictionary
header.size_zstd_dictionary
```

The query API can therefore load the dictionary directly from the database instead of requiring a separate dictionary file.

The compressed article and metadata payloads are intended to be decompressed using this dictionary.

---

# 7. Binary Data Types and Offset Rules

The database makes a strict distinction between **absolute offsets** and **section-relative offsets**.

This distinction is important throughout the format.

## Absolute offsets

Offsets stored in `DatabaseHeader` are absolute offsets from the beginning of the database file.

Examples:

```text
offset_content
offset_metadata
offset_titles
offset_pid_strings
```

Similarly, primary search-index level offsets stored in `IndexMetadata.level_offsets[]` are absolute database offsets.

For example:

```text
absolute level address =
    header.omni_search.level_offsets[level]
```

---

## Relative offsets

Offsets stored inside entity indexes refer to positions **inside their corresponding database sections**.

Examples:

```text
QIDIndexRow.offset
PIDIndexRow.title_offset
PIDIndexRow.desc_offset
QIDIndexRow.title_offset
```

For a QID payload:

```text
absolute payload offset =
    section base offset + relative payload offset
```

For example:

```text
header.offset_content + QIDIndexRow.offset
```

or:

```text
header.offset_metadata + QIDIndexRow.offset
```

For a PID string:

```text
header.offset_pid_strings + PIDIndexRow.title_offset
```

The header therefore defines the physical location of sections, while the indexes define locations within those sections.

---

## Packed Row Sizes

The fixed-size packed structures currently have the following sizes:

| Structure | Size |
| :--- | ---: |
| `AstronomicalRow` | 16 bytes |
| `AstronomicalSparseRow` | 16 bytes |
| `GlobeCoordinateRow` | 16 bytes |
| `GlobeCoordinateSparseRow` | 16 bytes |
| `TemporalRow` | 16 bytes |
| `TemporalSparseRow` | 16 bytes |
| `OmniRow` | `OMNI_SEARCH_TERM_SIZE + 8` bytes |
| `OmniSparseRow` | `OMNI_SEARCH_TERM_SIZE + 8` bytes |
| `QIDHashMapRow` | 6 bytes |
| `QIDIndexRow` | 18 bytes |
| `PIDHashMapRow` | 6 bytes |
| `PIDIndexRow` | 10 bytes |

The `__attribute__((packed))` annotation is important because these structures are used as fixed binary records. Padding inserted by the compiler must not change the on-disk layout.

---

## Summary

The current database architecture separates the problem into several layers:

```text
                         DATABASE
                            │
                     ┌──────┴──────┐
                     │    HEADER   │
                     └──────┬──────┘
                            │
        ┌───────────────────┼────────────────────┐
        │                   │                    │
        v                   v                    v
   SEARCH INDEXES      ENTITY INDEXES       DATA STORAGE
        │                   │                    │
        │             ┌─────┴─────┐       ┌────┴────┐
        │             │           │       │         │
        v             v           v       v         v
  Omni/Temporal/     QID         PID   Content   Metadata
  Astro/Globe       HashMap     HashMap
        │              │           │
        │              v           v
        │          QIDIndex    PIDIndex
        │              │           │
        │              │           └──> PID Strings
        │              │
        │              └──> Titles
        │
        └──> QID + Tags
                 │
                 v
          article / metadata
                 │
                 v
               ZSTD
```

The database is self-describing at the section level through its header. Primary search-index levels are located directly through `IndexMetadata`, while QID and PID indexes use compact section-relative offsets to locate their associated payloads and strings.

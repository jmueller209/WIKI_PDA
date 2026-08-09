# Database Architecture

This document outlines the internal structure and binary layout of the generated Wiki database (`data_base.bin`). The database is designed as a single, contiguous binary file optimized for low-memory environments, utilizing custom data structures and streaming compression.

---

## 1. High-Level Binary Layout

The entire database is bundled into a single binary file. While the exact byte-offsets depend on the generated content, the general structural sequence is as follows:

1. **Header / Magic Bytes** (Optional/Planned)
2. **Compression Dictionary** (ZSTD)
3. **Primary Search Indexes** (Flat k-trees)
4. **Entity Indexes** (QID & PID)
5. **Metadata Storage**
6. **Compressed Content Payload**

---

## 2. Primary Search Indexes (Flat k-trees)

To allow fast lookups without loading massive tree structures into RAM, the primary search indexes are implemented as **Flat k-trees**. These are laid out contiguously on disk, allowing the ESP32 to seek and read only the necessary nodes during a traversal.

### 2.1 Omni Search Index
* **Purpose:** The primary text-based index for searching concepts by name/title.
* **Structure Details:** *(To be documented)*

### 2.2 Temporal Search Index
* **Purpose:** Time-based index for querying concepts by specific dates or timespans (e.g., birth/death dates, historical events).
* **Structure Details:** *(To be documented)*

### 2.3 Global Search Index
* **Purpose:** Spatial index for querying locations based on standard Earth coordinates (Latitude/Longitude).
* **Structure Details:** *(To be documented)*

### 2.4 Astronomical Search Index
* **Purpose:** Celestial spatial index for querying stars, galaxies, and astronomical bodies using celestial coordinates.
* **Structure Details:** *(To be documented)*

---

## 3. Entity Indexes

Once a search index yields a match, it points to a specific entity identifier (QID or PID). These indexes map those identifiers to their physical data locations.

### 3.1 QID Search Index
* **Purpose:** Maps a Wikidata QID (e.g., `Q42`) to its corresponding rows. A single QID can point to multiple rows (e.g., the same article in different languages, or across different projects like Wikipedia vs. Wikiquotes).
* **Structure Details:** *(To be documented)*

### 3.2 PID Search Index
* **Purpose:** Stores property definitions (e.g., `P31` = "instance of"). Used to interpret and correctly render the compressed metadata.
* **Structure Details:** *(To be documented)*

---

## 4. Data Storage

This section contains the actual payloads returned to the user after a successful search and routing.

### 4.1 Metadata
* **Purpose:** Stores the wiki properties (PIDs) and tags associated with an article (e.g., coordinate data, categorization flags).
* **Structure Details:** *(To be documented)*

### 4.2 Content
* **Purpose:** The actual article text/HTML, stripped and formatted. 
* **Compression:** Stored in heavily compressed chunks using ZSTD.
* **Structure Details:** *(To be documented)*

---

## 5. Compression

### 5.1 ZSTD Dictionary
* **Purpose:** A pre-trained Z-Standard dictionary stored at the beginning of the file. By training a dictionary on Wiki data during the generation phase, the engine can decompress tiny data chunks (like individual articles) with extremely high compression ratios.
* **Structure Details:** *(To be documented)*

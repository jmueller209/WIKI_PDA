# Offline Wikipedia Database

## Summary
Here, you'll find the the structure and creation of a custom offline Wikipedia Database that can be saved on an SD card and queried using a simple microcontroller like the ESP32. Because of the very strict memory constraints, the focus lies purely on article's text without images. This repository contains various scripts for downloading Wikipedia articles and creating the database as well high performance code for performing queries.

## How does the online Wikipedia database work?
Contrary to popular belief, Wikipedia is not a single, massive encyclopedia, but rather a collection of many smaller encyclopedias that share a common platform. Each language has its own Wikipedia that is more or less independent from the others. This also means that two articles about the same concept can have different content depending on the language: The German article "Das Universum" is not a literal translation of the corresponding English article "Universe". While some articles are very similar in two languages, others might focus on completely different things. Since every language edition is its own project, the rules about how articles are written, administered, etc., are different in every language. This also leads to the situation where an article available in one language might not be available in another language. 

So, how are articles uniquely identified, and how is the German article about the Universe linked to the English one? It turns out that there is a project called Wikidata, which is a language-independent database that assigns each concept a unique identifier called a QID. For example, the QID of "Das Universum" is Q1, while the QID of "Universe" is also Q1, and so is the QID of "寰宇" (Classical Chinese). In fact, as of right now, there are 196 entries for Q1, meaning there are 196 different languages that have an article about the Universe. You can search for a specific QID on Wikidata using the following link: [https://www.wikidata.org/wiki/Q1](https://www.wikidata.org/wiki/Q1). Change Q1 to a different QID to search for a different concept. When you scroll all the way down the page, you'll find links to all Wikipedia entries with that QID. 

So, using a QID plus a language, we can uniquely identify a Wikipedia article. However, a user trying to find a specific concept probably does not know its QID. Fortunately, the <span style="color: red;">title</span> of an article can also be used as identifier, since it has to be unique within a given language. This is the reason why you might find article names that look like this:
- Mercury (planet)
- Mercury (element)
- Mercury (mythology)

Besides the title, an article can have a <span style="color: red;">label</span> and multiple <span style="color: red;">aliases</span>, which do not need to be unique. Here is an example of what information is associated with the QID (concept):

**Q308: Mercury (planet)**

| Language | Wikipedia Title (Unique) | Wikidata Label | Wikidata Aliases |
|---|---|---|---|
| **English** | Mercury (planet) | Mercury | Sol I, Planet Mercury, Hermes |
| **Spanish** | Mercurio (planeta) | Mercurio | planeta Mercurio |
| **German** | Merkur (Planet) | Merkur | *None standard* |
| ... | ... | ... | ... |

**Q925: Mercury (element)**

| Language | Wikipedia Title (Unique) | Wikidata Label | Wikidata Aliases |
|---|---|---|---|
| **English** | Mercury (element) | mercury | Hg, quicksilver, element 80 |
| **Spanish** | Mercurio (elemento) | mercurio | Hg, azogue, hidrargirio |
| **German** | Quecksilber | Quecksilber | Hg |
| ... | ... | ... | ... |

While the labels and aliases cannot be used as unique identifiers, they can help a lot when searching for articles as we the exact title of an article might not be known. 

---

## Database Structure
The goal of this project is to create an offline database of wikipedia articles that uses as little memory and processing power as possible. As I already mentioned above, articles in different languages might contain different contents. This was an incentive to not only create this offline database for one language only but make it as scalable as possible. It is easily possible to create an offline database containing every single Wikipedia article in all available languages (Without Images). The database consists of a series of binary files you will find in the `bin` folder once the database has been created succesfully. To start downloading Wikipedia articles and setting up the database, jump right into the [Setting up the Database](#setting-up-the-database) section of this README.

### 1. The Omni-Search Index (`omni_search_x.bin` and `omni_search_split_point.bin`)
**Purpose:** The single entry point for user text input. Contains every article title, alias, and lable for all downloaded languages, merged into one file.

**Sorting:** Strictly Alphabetical (UTF-8 byte values).

The Omni-Search Index acts as the first lookup-table that is searched when querying the database. It contains every single <span style="color: red;">alias</span>, <span style="color: red;">lable</span>, and <span style="color: red;">title</span> of every downloaded article. Each entry is a single record of exactly 72-bytes in length which are structured like this:

| Field Name | Type | Size | Format Specifier | Description |
| :--- | :--- | :--- | :--- | :--- |
| **Term (Title, Alias, or Lable)** | `char[64]` | 64 Bytes | `64s` | Null-padded UTF-8 search string |
| **Packed ID**| `uint32_t` | 4 Bytes | `I` | Q-ID (bits 0-30) + Type Flag (bit 31) |

The type flag is just an indicator whether the **term** is a title or not. This can be used to make the search algorithm more intelligent. The terms are sorted in alphabetical order. That way a simple binary search can be performed when the user types in a specific search term (Worst-case time complexity: $O(\log_2 n)$). This way the search algorithm can quickly find all possible matches (QIDs) for that term. The reason why the Omni-Search Index is split up into multiple files is that files on FAT32 formatted SD-Cards cannot exceed a maximum size of 4GB. Depending on the amount of languages you download, expect the Omni-Search Index to have a size of about 3-6GB.


### 2. The Vertical Master Map (`master_index.bin`)
**Purpose:** Connects a Q-ID to the physical SD card byte offset of the actual text. 
**Sorting:** Strictly Numerical (Lowest Q-ID to Highest).
**Scaling Strategy:** "Vertical Indexing." If an article exists in 5 languages, it has 5 tiny rows grouped together. If it exists in 1 language, it has 1 row.

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


## Setting up the Database
Test
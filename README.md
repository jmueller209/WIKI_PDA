# [Project Name]

A customable Wiki database generator + query API optimized for embedded and low power devices like the ESP32.

NOTE: This project is under construction and some of the features are either not thoroughly tested, have not been completely implemented or are have no documentation/not easy to use. Also note that this project has only been tested on Fedora so far, other Linux distributions will probably work without any or with minor tweaks. Native Windows and MacOS will not work right now as the database generator currently uses the GNU Coreutils sort function. I will implement another fallback sorting method (platform detection) in the future. For now, if you are on Windows you can run the generator inside WSL (Windows Subsystem for Linux). \\ Keep in mind that the query API might change in the future.  

---

## This project includes:
- ***[Database Generator](pathtodoc)***
- ***[API for querying the database](pathtodoc)***

## Future Core Functionality (not everything is implemented as of right now)
*   Support for **Wikipedia**, **Wiktionary**, (And perhaps **Wikiquotes**, **Wikiversity**, **Wikibooks**, **Wikisource** and **Wikivoyage**) in any or multiple languages.
*   **Customable Metadata** based on **Wiki** properties.
*   **Multi-Index Search:**:
    - Omni Search Index: Search for **Wikipedia** concepts (QIDs) by text.
    - Lexeme Search Index: Use the database as an offline Dictionary based on **Wiktionary**.
    - Property (PID) Search Index: Search for **Wiki** properties to process meta data.
    - Global Search Index: Search for **Wiki** concepts based on their globe coordinates (Might be useful in combination with Open Street Maps).
    - Astronomical Search Index: Search for **Wiki** concepts referring to Galaxies, Stars, Planets, Comets and more using their celestial coordinates.
    - Temporal Search Index: Search for **Wiki** concepts based on their data (e.g. data of     birth/death for people, start/end dates for historical concepts.
    - QID Search Index: Search **Wiki** concepts directly (used internally by the API to find corresponding to articles to for example an Omni Search term. Can be used Externally to implement automatic routing between articles using redirects).
* **Custom Search Tags** based on **Wiki** properties (PIDs). Eg. 'is_human', 'is_capital_city'...
* **Search Articles by language**
*   **Fast Lookups** optimized for SD Cards and low RAM usage using custom data structures and streaming compression so even large articles that do not fit into RAM can be read. 
*   **Z-Standard Compression** using a pre trained dictionary with customable performance metrics such as compression level and size.
*   **Interface to customize article processing** (Turn raw html into the format you'd like to have in your database while having the option to keep redirects between articles in tact)
*   **Interface to port the Query API to any platform**. 

## Current Functionality:
### Generator:
* Only **Wikipedia** supported right now (no **Wiktionary**, **Wikibooks**, ...)
* **Multi language support** for Wikipedia Articles
* **Omni Search Index**
* **Astronomical Search Index**
* **Temporal Search Index**
* **Global Search Index**
* **Wikipedia** Content and customizable metadata
* **Content compression** (no metadata compression right now) using ZSTD (customizable dictionary size, compression level, ...)
* All Indexes include **customizable search tags**
* **Partially multithreaded generator piepeline**


### Query API:
* Functionality for initializing a **DatabaseContext** and querying the following indexes based on your custom tags and language:
  - Omni Search Index
* Initialize a **DataStream** to read articles into a buffer.
* **DatabasePlatform** Interface to deine your own read_database_function() for your platform
* predefined **DatabasePlatform** desktops.
* Example program: **Wikipedia Terminal Reader**

## Priority Feature List (Please open an issue if you think there is something you would like to this list):
* Fixing bugs that I don't know yet
*  Add API support for **Temporal Search Index**, **Global Search Index**, **Astronomical Search Index**, direct **QID Search Index** **PID Search Index**.
*  **Wiktionary** support.
*  Decide on whether to properly support other wikis such as **Wikibooks**. This is a pain in the a** because other than **Wikipedia** articles, a Wiki Book for example consists of multiple chapters that need to be individually parsed and linked. This prevents me from using the same pipeline as for **Wikipedia** Articles and considering the small size of those other wikis compared to **Wikipedia**, it might not be worth it.
*  Making the generator work on windows (or maybe not because people should switch to Linux anyways)
*  Implementing a better default processing function for articles. Additionally, an easy way to turn redirects into QIDs would be nice so offline redirecting can be implemented (Using the QID Index).
  
*  Performance Improvements (focus on API).

---

## Architecture Overview

```mermaid
flowchart TD
    %% Color Palette Definition
    classDef search fill:#e3f2fd,stroke:#1e88e5,stroke-width:2px,color:#0d47a1
    classDef core fill:#f3e5f5,stroke:#8e24aa,stroke-width:2px,color:#4a148c
    classDef memory fill:#fff3e0,stroke:#fb8c00,stroke-width:2px,color:#e65100
    classDef storage fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:#1b5e20
    classDef tool fill:#ffebee,stroke:#e53935,stroke-width:2px,color:#b71c1c

    User([User Query])

    subgraph Phase1 [1. Query Resolution]
        OS[Omni Index<br>Text-based]:::search
        AS[Astro Index<br>Celestial]:::search
        TS[Temporal Index<br>Time-based]:::search
        CS[Geo Index<br>Coordinates]:::search
    end

    User --> OS
    User -.-> AS
    User -.-> TS
    User -.-> CS

    Q(((QID<br>Entity Hub))):::core
    
    OS --> Q
    AS --> Q
    TS --> Q
    CS --> Q

    subgraph Phase2 [2. O 1 Memory Routing]
        HM{Primary<br>HashMap}:::memory
        PTR[Memory Offset<br>& Row Count]:::memory
    end

    Q --> HM
    HM -->|Yields| PTR

    subgraph Phase3 [3. Multi-Project Routing]
        IDX[[QID Row Table]]:::memory
        R1(Row: enwiki)
        R2(Row: dewiki)
        R3(Row: enwikiquote)
    end

    PTR --> IDX
    IDX --> R1 & R2 & R3

    subgraph Phase4 [4. Storage Layer]
        MD[(Metadata)]:::storage
        DAT[(Encrypted &<br>Compressed Payload)]:::storage
    end

    R1 -->|Reads| MD
    R1 -->|Reads| DAT
    R2 -.-> MD & DAT
    R3 -.-> MD & DAT

    ZDICT>Pre-trained<br>Zstd Dictionary]:::tool
    ZDICT -.->|Decompresses| DAT
```

---

## Quick Start
*Note*: As of right now, this project has only been tested on Fedora. Other Linux distributions should work as well but it will break under Windows/Mac as of right now. 

### 0. Prerequisites



In order to build and run this project you will need `cargo` and `gcc` for compiling Rust and C:

*cargo*:
```bash
curl https://sh.rustup.rs -sSf | sh
```

*Development Tools including gcc:*

*Fedora:*
```
TODO: update system
sudo dnf group install "Development Tools"
```

*Ubuntu/Debian:*
```
sudo apt update
sudo apt install build-essential
```

*Arch:*
```
sudo pacman -Syu
sudo pacman -S gcc
```

### 1. Clone this repository and build
To get started, clone this directory using the following commands:
TODO: Change repo_name
```bash
git clone https://github.com/jmueller209/repo_name.git
cd repo_name
```
Once you are in this projects local directory you can compile the project using:
```
make build
```

### 2. Customize Configuration
Before running the database generator you can customize the configuration [here](config/config.toml). The file contains comments explaining what each setting does. A more detailed explanation for some of the setting will be given in the detailed [documentation](””). If you only want to get started quickly and not want to spend much time reading docs you might only want to consider the following settings:
*    `create_clobe_coordinate_search_index // supported by generator but not by API as of right now`
*    `create_temporal_search_index // supported by generator but not by API as of right now`
*    `create_astronomical_search_index // supported by generator but not by API as of right now`
*    `ram_limit_mb  // RAM your computer is allowed to use for the database generation`
*    `thread_count // Number of Threads/Cores the generator is allowed to use`

To configure which languages you want to include, take a look at the [The official Wikipedia Documentation](https://en.wikipedia.org/wiki/List_of_Wikipedias). Here you’ll find a table containing information about the Wikipdias in all available languages. Even though this documentation only talks about Wikipedia articles and not wiki books for example, the language codes used are the same. Open the [language configuration](config/languages.config) and add the language codes referring to the languages you want to include. 

*Note*: There might be several language codes for a given language that refer to different dialects or *difficulties*. E.g. There is `de` referring to standard German and `nds` referring to a specific low German dialect. 

### 3. Run The Generator
Generating the database is quite computationally expensive as we have to parse very large files. Just to give you a short summary of the most time consuming processing steps:
1. Downloading database dumps (Compressed Metadata archive: ~150GB, Compressed Data archives: ~1-50GB per wiki per language)
2. Uncompressing, parsing and processing Metadata Archieve (In its uncompressed form the Metadata Archieve is a ~1TB JSON file)
3. Uncompressing, parsing, processing and recompressing Data archives (For each Wikipedia article, wiki book, etc. we need to decompress its data in the dump file which contains raw html, process the data (remove html tags, etc.), compress the processed data and save it).

For testing (which I would recommend with the current state of the project) run the following command from the root of this repository:
```
make test-pipeline
```
This will first download the necessary database dumps and then start processing the data and generating the database with a very small number of articles to finish quickly. Note that the download will probably still take a long time but fortunately this will only be done once. You can change the config file (except the language config, the `wikis_to_include` and the `data_dir` setting) and run the `make test-pipeline` command again to generate a new database. You can run 
```
make clean
```
to clean up everything EXCEPT the the downloads. If you want to clean up everything including the downloads run
```
make purge
```
If you actually want to generate the entire database you can run
```
make resume
```
to keep generating from the last checkpoint or 
```
make restart-clean
```
to restart the database generation using the already existing downloads or
```
make restart-purge
```
to restart the database generation including removing the old downloads and download the latest data. Even though some of the steps that require heavy compute are multithreaded, generating the ENTIRE database will still take many hours. To avoid having to run your computer for 30 hours straight, the generator creates checkpoints from which it can resume when you run the `make resume` command. Downloads will be resumed automatically at the point where they were stopped. Keep in mind though that some of the larger processing steps that require heavy decompression must be run in one go. Exiting the generator early will not result in data corruption; however, a lot of progress might be lost. 

### 4. Test The Database
After generating the database (even if you only ran the `test-pipeline` command) you can run an example program that uses the query API to allow you to search through the database and read articles in the terminal. Just run
```
make test-db-api
```
If you want to run in DEBUG mode use
```
make test-db-api-debug
```
If you want to profile heap usage run
```
make test-db-api-valgrind
```


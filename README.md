# [Project Name]

A customable Wiki database generator optimized for embedded and low power devices like the ESP32.

**📖 [Full Documentation](link-to-docs)**

---

## Core Functionality
*   Support for **Wikipedia**, **Wiktionary**, **Wikiquotes**, **Wikiversity**, **Wikibooks**, **Wikisource** and **Wikivoyage** in any or multiple languages.
*   **Multi-Index Search:**:
    - Omni Search Index: Search wiki concepts by text.
    - Global Search Index (optional): Search wiki concepts based on their globe coordinates (Might be     useful in combination with Open Street Maps).
    - Astronomical Search Index (optional): Search astronomical objects like Galaxies, Stars, Planets, Comets and     more using their celestial coordinates.
    - Temporal Search Index (optional): Search Search wiki concepts based on their data (e.g. data of     birth/death for people, start/end dates for historical concepts.
*   **Fast Lookups** optimized for SD Cards and low RAM usage.
*   **Z-Standard Compression** using a pre trained dictionary with customable performance metrics.
*   **Customable Metadata** based on wiki properties.

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
*Note*: As of right now, this project has only been tested on Fedora. Other Linux distributions should work as well but it will probably break under Windows/Mac. 

*(See [Documentation](link) for detailed steps)*

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
```bash
git clone 
cd project-name
```

### 2. Setup Database & Dictionary
```bash
cargo run --release -- --generate-dictionary
cargo run --release -- --build-db /path/to/zim/files
```

### 3. Run
```bash
cargo run --release -- --serve
```

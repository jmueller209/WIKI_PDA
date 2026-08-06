# [Project Name]

> High-performance, offline Wikipedia retrieval engine with O(1) lookups and custom Zstd compression.

**📖 [Full Documentation & Detailed Guides](link-to-docs)**

---

## ⚡ Core Functionality

*   **Multi-Index Search:** Omni (text), Astro (celestial), Temporal (time), and Geo (coordinates).
*   **$O(1)$ Lookups:** Direct HashMap routing from Wikidata QID to memory offsets.
*   **Unified Routing:** Single QID points to all languages and projects (Wikipedia, Wiktionary, etc.).
*   **Optimized Storage:** Custom-trained Zstandard dictionary compression.

---

## 🏗️ Architecture Overview

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

## 🚀 Quick Start Outline

*(See [Documentation](link) for detailed steps)*

### 1. Build
```bash
git clone [https://github.com/yourusername/project-name.git](https://github.com/yourusername/project-name.git)
cd project-name
cargo build --release
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

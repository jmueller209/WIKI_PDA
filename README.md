```mermaid
flowchart LR
    %% Color Palette Definition
    classDef search fill:#e3f2fd,stroke:#1e88e5,stroke-width:2px,color:#0d47a1
    classDef core fill:#f3e5f5,stroke:#8e24aa,stroke-width:2px,color:#4a148c
    classDef memory fill:#fff3e0,stroke:#fb8c00,stroke-width:2px,color:#e65100
    classDef storage fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:#1b5e20
    classDef tool fill:#ffebee,stroke:#e53935,stroke-width:2px,color:#b71c1c

    User([👤 User Query])

    subgraph Phase1 ["1. Query Resolution"]
        direction TB
        OS["🔍 Omni Index (Text)"]:::search
        AS["⭐ Astro Index (Celestial)"]:::search
        TS["⏳ Temporal Index (Time)"]:::search
        CS["🌍 Geo Index (Coordinates)"]:::search
    end

    User --> OS
    User -.-> AS
    User -.-> TS
    User -.-> CS

    Q((("💠 QID (Entity Hub)"))):::core
    
    OS --> Q
    AS --> Q
    TS --> Q
    CS --> Q

    subgraph Phase2 ["2. O(1) Memory Routing"]
        direction LR
        HM{"Primary HashMap"}:::memory
        PTR["📝 Offset + Row Count"]:::memory
    end

    Q --> HM
    HM -->|Yields pointer| PTR

    subgraph Phase3 ["3. Multi-Project Routing"]
        direction TB
        IDX[["QID Row Table"]]:::memory
        R1("Row: enwiki")
        R2("Row: dewiki")
        R3("Row: enwikiquote")
    end

    PTR --> IDX
    IDX --> R1 & R2 & R3

    subgraph Phase4 ["4. Storage Layer"]
        direction TB
        MD[("🏷️ Metadata")]:::storage
        DAT[("📦 Encrypted & Compressed Payload")]:::storage
    end

    R1 -->|Reads| MD
    R1 -->|Reads| DAT
    R2 -.-> MD & DAT
    R3 -.-> MD & DAT

    ZDICT>"📚 Pre-trained Zstd Dictionary"]:::tool
    ZDICT -.->|Decompresses| DAT
```

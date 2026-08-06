flowchart TD
    %% Styling classes
    classDef search fill:#e1f5fe,stroke:#03a9f4,stroke-width:2px;
    classDef optional fill:#f3e5f5,stroke:#ce93d8,stroke-width:2px,stroke-dasharray: 5 5;
    classDef lookup fill:#fff3e0,stroke:#ff9800,stroke-width:2px;
    classDef routing fill:#e8f5e9,stroke:#4caf50,stroke-width:2px;
    classDef storage fill:#eceff1,stroke:#607d8b,stroke-width:2px;
    classDef dict fill:#ffebee,stroke:#f44336,stroke-width:2px;

    User((User))

    subgraph "1. Search Layer"
        OS[Omni Search<br/>(Term based)]:::search
        AS[Astronomical Search<br/>(Star coords)]:::optional
        TS[Temporal Search<br/>(Time based)]:::optional
        CS[Coordinate Search<br/>(Globe coords)]:::optional
    end

    User --> OS
    User -.-> AS
    User -.-> TS
    User -.-> CS

    OS --> QID([Resolved QID])
    AS --> QID
    TS --> QID
    CS --> QID

    subgraph "2. O(1) QID Lookup"
        QID --> HM{{HashMap Index}}:::lookup
        HM -->|Yields| Pointers[Memory Offset <br/>+ Number of Rows]:::lookup
    end

    subgraph "3. Article Routing Index"
        Pointers --> AI[QID Row Table]:::routing
        AI --> Row1[Row: Wikipedia - English]:::routing
        AI --> Row2[Row: Wikipedia - German]:::routing
        AI --> Row3[Row: Wikiquote - English]:::routing
        AI -.-> RowN[Row: ...]:::routing
    end

    subgraph "4. Data & Metadata Storage"
        Row1 -->|Metadata Pointer| Meta[(Metadata)]:::storage
        Row1 -->|Data Pointer| Data[(Compressed Data)]:::storage
        
        Row2 --> Meta
        Row2 --> Data
        
        ZSTD[[Custom Pretrained<br/>Zstandard Dictionary]]:::dict
        ZSTD -.->|Decompresses| Data
    end

# Offline Wikipedia Database


```mermaid
graph TD;
    A[User] -->|Uploads File| B(Web Server);
    B --> C{File Type?};
    C -->|Image| D[Image Compressor];
    C -->|Text| E[Text Parser];
    D --> F[(Database)];
    E --> F; read-head and loading a single fixed-width struct at a time (e.g., max 72 Bytes).


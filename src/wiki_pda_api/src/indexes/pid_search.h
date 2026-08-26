#include <stdint.h>

/**
 * @brief O(1) Lookup Table für PIDs (properties_hashmap.bin).
 * Adresse = (PID - 1) * 6 Bytes.
 */
typedef struct __attribute__((packed)) {
    uint32_t start_index; /**< Zeile in der properties_index.bin, bei der die Übersetzungen starten */
    uint16_t entry_count; /**< Anzahl der verfügbaren Sprachen (Zeilen) für diese PID. 0 = PID existiert nicht. */
} PropertyHashMapRow; // Exakt 6 Bytes (Genau wie bei QID!)

/**
 * @brief Die eigentlichen Einträge für jede Sprache (properties_index.bin).
 * Die Hashmap sagt uns, wie viele dieser Zeilen wir am Stück lesen müssen.
 */
typedef struct __attribute__((packed)) {
    uint16_t lang_id;         /**< Mappt auf eine language_dictionary.txt (z.B. 0=en, 1=de) */
    uint32_t title_offset;    /**< Offset in der properties_strings.bin für den Namen */
    uint32_t desc_offset;     /**< Offset in der properties_strings.bin für die Beschreibung */
} PropertyIndexRow; // Exakt 10 Bytes (Vorher 16 Bytes, massiv Platz gespart!)

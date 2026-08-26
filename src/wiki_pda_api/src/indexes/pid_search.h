#ifndef QID_SEARCH_H
#define QID_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../common/generated_database_constants.h"
#include "../../include/database_platform.h"

/**
 * @brief O(1) Lookup Table für PIDs (properties_hashmap.bin).
 * Adresse = (PID - 1) * 6 Bytes.
 */
typedef struct __attribute__((packed)) {
    uint32_t start_index; /**< Zeile in der pid_index.bin, bei der die Übersetzungen starten */
    uint16_t entry_count; /**< Anzahl der verfügbaren Sprachen (Zeilen) für diese PID. 0 = PID existiert nicht. */
} PropertyHashMapRow; // Exakt 6 Bytes

/**
 * @brief Die eigentlichen Einträge für jede Sprache (pid_index.bin).
 * Die Hashmap sagt uns, wie viele dieser Zeilen wir am Stück lesen müssen.
 */
typedef struct __attribute__((packed)) {
    uint16_t project_id;      /**< Mappt auf die globale project_id (z.B. 2=dewiki) */
    uint32_t title_offset;    /**< Relativer Offset in der pid_strings.bin für den Namen */
    uint32_t desc_offset;     /**< Relativer Offset in der pid_strings.bin für die Beschreibung */
} PropertyIndexRow; // Exakt 10 Bytes


// ============================================================================
// FUNKTIONS-DEKLARATIONEN
// ============================================================================

/**
 * @brief Liest den O(1) Hashmap-Eintrag für eine gegebene PID.
 * 
 * @param pid Die gesuchte Property-ID (z.B. 31 für P31).
 * @param out_row Pointer auf das Struct, das mit den gelesenen Daten gefüllt wird.
 * @param platform Die Plattform-API für den Dateizugriff (SD-Karte).
 * @return true wenn die PID existiert (entry_count > 0), false wenn nicht (Lücke/Padding).
 */
bool get_pid_hashmap_entry(uint32_t pid, PropertyHashMapRow* out_row, DatabasePlatform platform);

/**
 * @brief Sucht in der pid_index.bin nach der exakten Sprache (project_id) für eine PID.
 * 
 * @param start_index Der `start_index` aus der PropertyHashMapRow.
 * @param entry_count Der `entry_count` aus der PropertyHashMapRow.
 * @param target_project_id Die ID der gesuchten Sprache (z.B. 2 für Deutsch).
 * @param out_row Pointer auf das Struct, das mit den String-Offsets gefüllt wird.
 * @param platform Die Plattform-API für den Dateizugriff.
 * @return true wenn eine Übersetzung in dieser Sprache gefunden wurde, sonst false.
 */
bool get_pid_translation_row(uint32_t start_index, uint16_t entry_count, uint16_t target_project_id, PropertyIndexRow* out_row, DatabasePlatform platform);

/**
 * @brief Liest einen null-terminierten String (Titel oder Beschreibung) aus dem String-Pool.
 * Fügt intern den absoluten Offset OFFSETS_PID_STRINGS hinzu.
 * 
 * @param string_offset Der relative Offset aus der PropertyIndexRow.
 * @param out_buffer Pointer auf den Puffer, in den der String geschrieben wird.
 * @param max_length Die maximale Größe des Puffers.
 * @param platform Die Plattform-API für den Dateizugriff.
 * @return true bei erfolgreichem Lesen.
 */
bool get_pid_string(uint32_t string_offset, char* out_buffer, size_t max_length, DatabasePlatform platform);


#endif

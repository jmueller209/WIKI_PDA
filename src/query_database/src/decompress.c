#include "database_constants.h"
#include "database_io.h"
#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <zstd.h>

#ifdef DEBUG_MODE
    // Wenn DEBUG_MODE definiert ist, wird DEBUG_PRINT zu einem printf
    #define DEBUG_PRINT(fmt, ...) printf("[DEBUG] " fmt, ##__VA_ARGS__)
#else
    // Wenn DEBUG_MODE NICHT definiert ist, entfernt der Compiler den Befehl komplett
    #define DEBUG_PRINT(fmt, ...) do {} while (0)
#endif

bool load_zstd_dictionary(uint8_t** out_dictionary, uint64_t* out_length) {
    if (out_dictionary == NULL || out_length == NULL) {
        return false;
    }

    uint64_t dict_size = SIZES_ZSTD_DICTIONARY;
    uint64_t dict_offset = OFFSETS_ZSTD_DICTIONARY;

    if (dict_size == 0) {
        return false;
    }

    uint8_t* buffer = (uint8_t*)malloc(dict_size);
    if (buffer == NULL) {
        return false;
    }

    if (!platform_database_read(dict_offset, buffer, dict_size)) {
        free(buffer);
        return false;
    }

    *out_dictionary = buffer;
    *out_length = dict_size;

    return true;
}

bool decompress_data(
    const uint8_t* compressed_data,
    uint32_t compressed_length,
    const uint8_t* dictionary,
    uint64_t dict_length,
    char** out_decompressed_text,
    uint32_t* out_decompressed_length
) {
    // 1. Guard clauses (Check ob wir überhaupt Platz für unsere 4 Bytes haben)
    if (compressed_data == NULL || dictionary == NULL || 
        out_decompressed_text == NULL || out_decompressed_length == NULL || 
        compressed_length <= 4) {
        
        DEBUG_PRINT("ZSTD: Ein übergebener Pointer war NULL oder Daten zu kurz (<=4 Bytes).\n");
        return false;
    }

    // 2. Die echte Artikelgröße aus den ersten 4 Bytes extrahieren (Little Endian!)
    uint32_t uncompressed_size = compressed_data[0] | 
                                (compressed_data[1] << 8) | 
                                (compressed_data[2] << 16) | 
                                (compressed_data[3] << 24);

    // 3. Den Pointer um 4 Bytes nach vorne schieben, um die echten ZSTD-Daten zu treffen
    const uint8_t* actual_zstd_data = compressed_data + 4;
    uint32_t actual_zstd_length = compressed_length - 4;

    // 4. Speicher exakt passend reservieren (+1 für '\0')
    char* text_buffer = (char*)malloc((size_t)uncompressed_size + 1);
    if (text_buffer == NULL) {
        DEBUG_PRINT("ZSTD: malloc fehlgeschlagen für %u Bytes.\n", uncompressed_size + 1);
        return false;
    }

    ZSTD_DCtx* dctx = ZSTD_createDCtx();
    if (dctx == NULL) {
        DEBUG_PRINT("ZSTD: Fehler beim Erstellen des ZSTD-Kontexts.\n");
        free(text_buffer);
        return false;
    }

    // 5. Dekomprimieren (Wichtig: Wir übergeben actual_zstd_data und actual_zstd_length)
    size_t actual_decompressed_size = ZSTD_decompress_usingDict(
        dctx, 
        text_buffer, uncompressed_size, 
        actual_zstd_data, actual_zstd_length, 
        dictionary, dict_length
    );

    ZSTD_freeDCtx(dctx);

    if (ZSTD_isError(actual_decompressed_size)) {
        DEBUG_PRINT("ZSTD Dekomprimierungs-Fehler: %s\n", ZSTD_getErrorName(actual_decompressed_size));
        free(text_buffer);
        return false;
    }

    // C-String sauber terminieren
    text_buffer[actual_decompressed_size] = '\0';
    
    *out_decompressed_text = text_buffer;
    *out_decompressed_length = (uint32_t)actual_decompressed_size;

    DEBUG_PRINT("ZSTD: Erfolgreich entpackt (Daten+Header: %u Bytes -> Entpackt: %zu Bytes).\n", 
                compressed_length, actual_decompressed_size);

    return true;
}

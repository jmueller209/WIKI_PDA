#if defined(TEENSYDUINO)
#include <Arduino.h>
#include <stdint.h>
#include "SdFat.h"
#include <string.h>
#include "../../include/wiki_pda_platforms.h"
#include "../../include/wiki_pda_options.h"

typedef struct {
    SdCard* card;
    uint32_t start_sector;
} TeensySdInternalContext;

// Read the Master Boot Record to find where the second partition begins.
static uint32_t parse_partition2_lba(SdCard* card) {
    uint8_t mbr[SD_SECTOR_SIZE];
    if (!card->readSector(0, mbr)) return 0;

    if (mbr[510] != 0x55 || mbr[511] != 0xAA) return 0;

    return (uint32_t)mbr[470] | ((uint32_t)mbr[471] << 8) | 
           ((uint32_t)mbr[472] << 16) | ((uint32_t)mbr[473] << 24);
}

static bool teensy_sd_read(uint64_t offset, uint8_t* buf, uint32_t len, void* user_data) {
    TeensySdInternalContext* ctx = (TeensySdInternalContext*)user_data;
    if (!ctx || !ctx->card) return false;

    uint64_t abs_byte_offset = ((uint64_t)ctx->start_sector * SD_SECTOR_SIZE) + offset;
    uint32_t current_sector = (uint32_t)(abs_byte_offset / SD_SECTOR_SIZE);
    uint32_t sector_offset = (uint32_t)(abs_byte_offset % SD_SECTOR_SIZE);

    uint8_t temp_block[SD_SECTOR_SIZE];
    uint32_t bytes_remaining = len;
    uint8_t* out_ptr = buf;

    if (sector_offset != 0) {
        if (!ctx->card->readSector(current_sector, temp_block)) return false;

        uint32_t chunk = SD_SECTOR_SIZE - sector_offset;
        if (chunk > bytes_remaining) chunk = bytes_remaining;

        memcpy(out_ptr, temp_block + sector_offset, chunk);
        out_ptr += chunk;
        bytes_remaining -= chunk;
        current_sector++;
    }

    uint32_t full_sectors = bytes_remaining / SD_SECTOR_SIZE;
    if (full_sectors > 0) {
        if (!ctx->card->readSectors(current_sector, out_ptr, full_sectors)) return false;

        out_ptr += (full_sectors * SD_SECTOR_SIZE);
        bytes_remaining -= (full_sectors * SD_SECTOR_SIZE);
        current_sector += full_sectors;
    }

    if (bytes_remaining > 0) {
        if (!ctx->card->readSector(current_sector, temp_block)) return false;
        memcpy(out_ptr, temp_block, bytes_remaining);
    }

    return true;
}

DatabasePlatform platform_teensy(void* file_handle) {
    // 1. Cast den Opaque Pointer sicher in deinen C++ SdCard* um
    SdCard* card = (SdCard*)file_handle;

    DatabasePlatform platform = {0};
    if (!card) return platform;

    TeensySdInternalContext* ctx = new TeensySdInternalContext();
    ctx->card = card;
    ctx->start_sector = parse_partition2_lba(card);

    if (ctx->start_sector == 0) {
        delete ctx;
        return platform; 
    }

    #if WPDA_MAGIC_LENGTH > 0
    {
        uint8_t first_sector[SD_SECTOR_SIZE];
        if (!card->readSector(ctx->start_sector, first_sector)) {
            delete ctx;
            return platform;
        }

        if (memcmp(first_sector, WPDA_MAGIC, WPDA_MAGIC_LENGTH) != 0) {
            delete ctx;
            return platform;
        }
    }
    #endif

    platform.read_fn = teensy_sd_read;
    platform.user_data = (void*)ctx;
    return platform;
}

#endif // TEENSYDUINO

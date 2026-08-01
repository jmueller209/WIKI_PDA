#ifndef DATABASE_IO_H
#define DATABASE_IO_H

#include <stdint.h>
#include <stdbool.h>

// DECLARATION ONLY. 
// The C library will call this, but the platform (PC/ESP32) MUST implement it.
extern bool platform_database_read(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes);

#endif // DATABASE_IO_H

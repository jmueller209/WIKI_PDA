#ifndef DATABASE_PLATFORM_H
#define DATABASE_PLATFORM_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

/**
 * @brief Function signature for reading raw bytes from the database storage medium.
 *
 * This abstraction allows the core database engine to remain platform-agnostic.
 * The implementation must handle the actual physical hardware read (e.g., POSIX fread
 * on desktop, or SPI/SD card reads on an ESP32).
 *
 * @param absolute_offset The exact byte offset from the beginning of the database file.
 * @param buffer Pointer to the memory buffer where the read data should be written.
 * @param num_bytes The exact number of bytes to read.
 * @param user_data An opaque pointer to platform-specific context (e.g., a FILE* handle).
 *
 * @return true if exactly `num_bytes` were successfully read, false if an error or EOF occurred.
 */
typedef bool (*DatabaseReadFn)(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes, void* user_data);

/**
 * @brief An abstraction layer binding the database engine to the underlying hardware or OS.
 *
 * Create an instance of this struct and pass it to db_init() to tell the search engine
 * how to fetch data from your specific storage backend.
 */
typedef struct {
    /** 
     * @brief Pointer to the custom platform-specific read function.
     */
    DatabaseReadFn read_fn;

    /**
     * @brief Context pointer passed back into every call of read_fn.
     * Commonly used to store file handles (e.g., FILE* or an SD card object reference)
     * so the read function knows which file to read from.
     */
    void* user_data;
} DatabasePlatform;

#endif

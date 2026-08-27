/**
 * @file platform_providers.h
 * @brief Storage abstraction layer and built-in platform providers.
 */

#ifndef PLATFORM_PROVIDERS_H
#define PLATFORM_PROVIDERS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

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
 * Create an instance of this struct and pass it to your initialization function to tell 
 * the search engine how to fetch data from your specific storage backend.
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


// ============================================================================
// BUILT-IN PLATFORM PROVIDERS
// ============================================================================

/**
 * @brief Creates a platform interface for Desktop/PC environments.
 * @param f An opened standard C file handle (must be opened in "rb" mode).
 * @return A populated DatabasePlatform struct ready to be passed to the database.
 */
DatabasePlatform platform_desktop(FILE* f);

/**
 * @brief Creates a platform interface for the ESP32.
 * @param file_handle A pointer to your opened C++ File or SD object.
 * @return A populated DatabasePlatform struct.
 */
DatabasePlatform platform_esp32(void* file_handle);

/**
 * @brief Creates a platform interface for the Teensy.
 * @param file_handle A pointer to your opened C++ File or SD object.
 * @return A populated DatabasePlatform struct.
 */
DatabasePlatform platform_teensy(void* file_handle);

#ifdef __cplusplus
}
#endif

#endif // PLATFORM_PROVIDERS_H

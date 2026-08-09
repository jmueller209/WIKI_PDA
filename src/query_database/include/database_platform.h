#ifndef DATABASE_PLATFORM_H
#define DATABASE_PLATFORM_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

typedef bool (*DatabaseReadFn)(uint64_t absolute_offset, uint8_t* buffer, uint32_t num_bytes, void* user_data);

typedef struct {
    DatabaseReadFn read_fn;
    void* user_data;
} DatabasePlatform;

#endif

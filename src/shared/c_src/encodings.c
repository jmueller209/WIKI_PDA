#include "encodings.h"
#include <stddef.h> 
#include <stdio.h>
#include <stdint.h>
#include <inttypes.h>

int64_t encode_time(const char* iso_str) {
    if (iso_str == NULL || iso_str[0] == '\0') {
        return -1;
    }

    char sign;
    int64_t year = 0, month = 0, day = 0;

    if (sscanf(iso_str, "%c%" SCNd64 "-%" SCNd64 "-%" SCNd64, &sign, &year, &month, &day) < 4) {
        return -1;
    }

    if (sign == '-') {
        year = -year;
    }

    int64_t sortable_date = (year * 10000) + (month * 100) + day;

    return sortable_date;
}

static inline uint64_t split_bits(uint32_t a) {
    uint64_t x = a;
    x = (x | (x << 16)) & 0x0000FFFF0000FFFFULL;
    x = (x | (x <<  8)) & 0x00FF00FF00FF00FFULL;
    x = (x | (x <<  4)) & 0x0F0F0F0F0F0F0F0FULL;
    x = (x | (x <<  2)) & 0x3333333333333333ULL;
    x = (x | (x <<  1)) & 0x5555555555555555ULL;
    return x;
}

int64_t encode_globe_coordinates(double lat, double lon) {
    uint32_t x = (uint32_t)((lat + 90.0) * 10000000.0);
    uint32_t y = (uint32_t)((lon + 180.0) * 10000000.0);
    uint64_t z_code = (split_bits(x) << 1) | split_bits(y);
    return (int64_t)z_code;
}

int64_t encode_astronomical_position(double dec, double ra) {
    uint32_t x = (uint32_t)((dec + 90.0) * 10000000.0);
    uint32_t y = (uint32_t)(ra * 10000000.0);
    uint64_t z_code = (split_bits(x) << 1) | split_bits(y);
    return (int64_t)z_code;
}

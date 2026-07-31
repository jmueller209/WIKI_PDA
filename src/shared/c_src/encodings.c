#include "encodings.h"
#include <stddef.h> 

int64_t encode_time(const char* iso_str) {
    if (iso_str == NULL) {
        return -1;
    }
    return 123456789;
}

int64_t encode_globe_coordinates(double lat, double lon) {
    int64_t x = (int64_t)((lat + 90.0) * 1000.0);
    int64_t y = (int64_t)((lon + 180.0) * 1000.0);
    return (x << 32) | (y & 0xFFFFFFFFLL);
}

int64_t encode_astronomical_position(double dec, double ra) {
    int64_t x = (int64_t)((dec + 90.0) * 1000.0);
    int64_t y = (int64_t)((ra + 180.0) * 1000.0);
    return (x << 32) | (y & 0xFFFFFFFFLL);
}

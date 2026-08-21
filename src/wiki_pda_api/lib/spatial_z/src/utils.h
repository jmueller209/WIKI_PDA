#ifndef INTERNAL_H
#define INTERNAL_H

#include <stdint.h>
#include "../include/spatial_z.h"

#define M_PI 3.14159265358979323846

static inline uint64_t spread_bits_32_to_64(uint32_t v) {
    uint64_t x = v;
    x = (x | (x << 16)) & 0x00000ffff0000ffffULL;
    x = (x | (x << 8))  & 0x00ff00ff00ff00ffULL;
    x = (x | (x << 4))  & 0x0f0f0f0f0f0f0f0fULL;
    x = (x | (x << 2))  & 0x3333333333333333ULL;
    x = (x | (x << 1))  & 0x5555555555555555ULL;
    return x;
}

static inline uint32_t compact_bits_64_to_32(uint64_t x) {
    x &= 0x5555555555555555ULL;
    x = (x | (x >> 1))  & 0x3333333333333333ULL;
    x = (x | (x >> 2))  & 0x0f0f0f0f0f0f0f0fULL;
    x = (x | (x >> 4))  & 0x00ff00ff00ff00ffULL;
    x = (x | (x >> 8))  & 0x0000ffff0000ffffULL;
    x = (x | (x >> 16)) & 0x00000000ffffffffULL;
    return (uint32_t)x;
}


uint32_t lat_to_grid(double lat, SpatialzCtx ctx);
uint32_t lon_to_grid(double lon, SpatialzCtx ctx);
double grid_to_lat(uint32_t grid_y, SpatialzCtx ctx);
double grid_to_lon(uint32_t grid_x, SpatialzCtx ctx);


#endif // INTERNAL_H

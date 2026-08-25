#ifndef INTERNAL_H
#define INTERNAL_H

#include <stdint.h>
#include "../include/spatial_z.h"

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

void to_internal_sphere(float user_y, float user_x, SpatialzCtx ctx, 
                        float *internal_y, float *internal_x);

void to_user_space(float internal_y, float internal_x, SpatialzCtx ctx, 
                   float *user_y, float *user_x);


uint32_t axis1_to_grid(float internal_y);
uint32_t axis2_to_grid(float internal_x);

float grid_to_axis1(uint32_t grid_y);
float grid_to_axis2(uint32_t grid_x);


#endif // INTERNAL_H

#include "../include/spatial_z.h"
#include "utils.h"

uint64_t spatial_encode(
    float axis1,
    float axis2,
    SpatialzCtx ctx
) {
    float internal_y, internal_x;

    to_internal_sphere(axis1, axis2, ctx, &internal_y, &internal_x);

    uint32_t grid_y = axis1_to_grid(internal_y);
    uint32_t grid_x = axis2_to_grid(internal_x);

    uint64_t encoded_x = spread_bits_32_to_64(grid_x);
    uint64_t encoded_y = spread_bits_32_to_64(grid_y);

    return encoded_x | (encoded_y << 1);
}

bool spatial_decode(
    uint64_t code,
    float* out_axis1,
    float* out_axis2,
    SpatialzCtx ctx
) {
    if (!out_axis1 || !out_axis2)
        return false;

    uint32_t grid_x = compact_bits_64_to_32(code);
    uint32_t grid_y = compact_bits_64_to_32(code >> 1);

    float internal_y = grid_to_axis1(grid_y);
    float internal_x = grid_to_axis2(grid_x);

    to_user_space(internal_y, internal_x, ctx, out_axis1, out_axis2);

    return true;
}

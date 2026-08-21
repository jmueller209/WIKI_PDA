#include "../include/spatial_z.h"
#include "utils.h"

uint64_t spatial_encode(double lat, double lon, SpatialzCtx ctx) {
    uint32_t grid_y = lat_to_grid(lat, ctx);
    uint32_t grid_x = lon_to_grid(lon, ctx);
    uint64_t encoded_x = spread_bits_32_to_64(grid_x);
    uint64_t encoded_y = spread_bits_32_to_64(grid_y);
    return encoded_x | (encoded_y << 1);
}

bool spatial_decode(uint64_t code, double* out_lat, double* out_long, SpatialzCtx ctx) {
    if (!out_lat || !out_long) return false;
    uint32_t grid_x = compact_bits_64_to_32(code);
    uint32_t grid_y = compact_bits_64_to_32(code >> 1);
    *out_lat = grid_to_lat(grid_y, ctx);
    *out_long = grid_to_lon(grid_x, ctx);
    return true;
}

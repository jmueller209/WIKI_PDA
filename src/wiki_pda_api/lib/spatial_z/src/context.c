#include "../include/spatial_z.h"

SpatialzCtx spatial_create_ctx(double min_lat, double max_lat, double min_lon, double max_lon, double unit_length) {
    SpatialzCtx ctx = {
        .min_lat = min_lat,
        .max_lat = max_lat,
        .min_long = min_lon,
        .max_long = max_lon,
        .unit_length = unit_length
    };
    return ctx;
}

SpatialzCtx spatial_create_earth_ctx(void) {
    return spatial_create_ctx(-90.0, 90.0, -180.0, 180.0, 111.3195);
}


SpatialzCtx spatial_create_celestial_ctx(void) {
    return spatial_create_ctx(-90.0, 90.0, -180.0, 180.0, 1.0);
}

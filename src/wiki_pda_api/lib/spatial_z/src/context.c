#include "../include/spatial_z.h"

SpatialzCtx spatial_create_ctx(
    float min_axis1,
    float min_axis2,
    float unit_length
) {
    SpatialzCtx ctx = {
        .min_axis1 = min_axis1,
        .min_axis2 = min_axis2,
        .unit_length = unit_length
    };

    return ctx;
}

SpatialzCtx spatial_create_earth_ctx(void) {
    return spatial_create_ctx(-90, -180, 111.3195);
}


SpatialzCtx spatial_create_celestial_ctx(void) {
    return spatial_create_ctx(-90 , 0, 1.0);
}

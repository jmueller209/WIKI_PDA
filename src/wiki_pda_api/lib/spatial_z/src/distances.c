#include "../include/spatial_z.h"
#include "utils.h"
#include <math.h>


CompareCtx spatial_create_compare_ctx(double center_lat, double center_lon, double radius, bool is_spherical, SpatialzCtx spatialCtx) {
    CompareCtx ctx;
    ctx.spatialCtx = spatialCtx;

    if (!is_spherical) {
        ctx.local.lat = center_lat;
        ctx.local.lon = center_lon;
        ctx.local.km_per_deg_lat = spatialCtx.unit_length;
        ctx.local.km_per_deg_lon = spatialCtx.unit_length * cos(center_lat * (M_PI / 180.0));
        ctx.local.radius_squared = radius * radius;
    } else {
        ctx.spherical.center_lat_rad = center_lat * (M_PI / 180.0);
        ctx.spherical.center_lon_rad = center_lon * (M_PI / 180.0);
        ctx.spherical.cos_center_lat = cos(ctx.spherical.center_lat_rad);

        double R = spatialCtx.unit_length * (180.0 / M_PI);
        double angular_radius = radius / (2.0 * R);
        double s = sin(angular_radius);
        ctx.spherical.max_haversine_a = s * s;
    }

    return ctx;
}

bool spatial_code_is_in_local_radius(uint64_t code, CompareCtx ctx) {
    double lat1;
    double lon1;
    if (!spatial_decode(code, &lat1, &lon1, ctx.spatialCtx)) {
        return false;
    }

    double dlon = lon1 - ctx.local.lon;
    while (dlon < -180.0) dlon += 360.0;
    while (dlon > 180.0) dlon -= 360.0;

    double dx = dlon * ctx.local.km_per_deg_lon;
    double dy = (lat1 - ctx.local.lat) * ctx.local.km_per_deg_lat;

    return ((dx * dx + dy * dy) <= ctx.local.radius_squared);
}

bool spatial_code_is_in_spherical_radius(uint64_t code, CompareCtx ctx) {
    double row_lat;
    double row_lon;
    if (!spatial_decode(code, &row_lat, &row_lon, ctx.spatialCtx)) {
        return false;
    }

    double row_lat_rad = row_lat * (M_PI / 180.0);
    double row_lon_rad = row_lon * (M_PI / 180.0);

    double dlat = row_lat_rad - ctx.spherical.center_lat_rad;
    double dlon = row_lon_rad - ctx.spherical.center_lon_rad;

    double sin_dlat = sin(dlat * 0.5);
    double sin_dlon = sin(dlon * 0.5);

    double a = (sin_dlat * sin_dlat) + 
               (ctx.spherical.cos_center_lat * cos(row_lat_rad) * sin_dlon * sin_dlon);

    return a <= ctx.spherical.max_haversine_a;
}

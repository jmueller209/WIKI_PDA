#include "../include/spatial_z.h"
#include "utils.h"
#include <stddef.h>
#include <math.h>
#include <stdbool.h>

static inline float deg_to_rad(float value)
{
    return value * (SPATIALZ_PI / 180.0f);
}


static inline float surface_radius(const SpatialzCtx *ctx)
{
    return ctx->unit_length * (180.0f / SPATIALZ_PI);
}

static inline void latlon_to_vector(float lat_rad, float lon_rad, float out_v[3])
{
    const float cos_lat = cosf(lat_rad);
    out_v[0] = cos_lat * cosf(lon_rad);
    out_v[1] = cos_lat * sinf(lon_rad);
    out_v[2] = sinf(lat_rad);
}

static inline float chord_distance_sq(const float v1[3], const float v2[3])
{
    const float dx = v1[0] - v2[0];
    const float dy = v1[1] - v2[1];
    const float dz = v1[2] - v2[2];
    return dx * dx + dy * dy + dz * dz;
}


CompareCtx spatial_create_compare_ctx(
    float center_axis1,
    float center_axis2,
    float radius,
    SpatialzCtx spatialCtx)
{
    CompareCtx ctx = {0};
    ctx.spatialCtx = spatialCtx;

    ctx.center_axis1 = center_axis1;
    ctx.center_axis2 = center_axis2;
    ctx.radius = radius;

    float int_y, int_x;
    to_internal_sphere(center_axis1, center_axis2, spatialCtx, &int_y, &int_x);

    const float lat_rad = deg_to_rad(int_y - 90.0f);
    const float lon_rad = deg_to_rad(int_x);

    latlon_to_vector(lat_rad, lon_rad, ctx.center_v);

    const float sphere_radius = surface_radius(&spatialCtx);
    float radius_radians = sphere_radius > 0.0f ? radius / sphere_radius : 0.0f;
    if (radius_radians > SPATIALZ_PI) {
        radius_radians = SPATIALZ_PI;
    }
    const float half_chord = sinf(radius_radians * 0.5f);

    ctx.radius_chord_sq = 4.0f * half_chord * half_chord;

    return ctx;
}

float spatial_code_is_in_radius(
    uint64_t code,
    const CompareCtx *ctx)
{
    if (ctx == NULL) return -1.0f;

    float axis1, axis2;

    if (!spatial_decode(code, &axis1, &axis2, ctx->spatialCtx)) {
        return -1.0f;
    }

    float int_y, int_x;
    to_internal_sphere(axis1, axis2, ctx->spatialCtx, &int_y, &int_x);

    float sample_v[3];
    latlon_to_vector(
        deg_to_rad(int_y - 90.0f),
        deg_to_rad(int_x),
        sample_v
    );

    const float dist_sq = chord_distance_sq(ctx->center_v, sample_v);

    if (dist_sq > ctx->radius_chord_sq) {
        return -1.0f;
    }

    return dist_sq;
}


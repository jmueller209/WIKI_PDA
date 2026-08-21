#include <math.h>
#include <stdint.h>
#include "utils.h"
#include "../include/spatial_z.h"

#define GRID_MAX_UINT 4294967295.0

uint32_t lat_to_grid(double lat, SpatialzCtx ctx)
{
    if (lat < ctx.min_lat) lat = ctx.min_lat;
    if (lat > ctx.max_lat) lat = ctx.max_lat;

    const double range = ctx.max_lat - ctx.min_lat;
    const double norm = (range == 0.0)
        ? 0.0
        : (lat - ctx.min_lat) / range;

    if (norm <= 0.0) return 0U;
    if (norm >= 1.0) return UINT32_MAX;
    return (uint32_t)(norm * GRID_MAX_UINT);
}

uint32_t lon_to_grid(double lon, SpatialzCtx ctx)
{
    if (lon < ctx.min_long) lon = ctx.min_long;
    if (lon > ctx.max_long) lon = ctx.max_long;

    const double range = ctx.max_long - ctx.min_long;
    const double norm = (range == 0.0)
        ? 0.0
        : (lon - ctx.min_long) / range;

    if (norm <= 0.0) return 0U;
    if (norm >= 1.0) return UINT32_MAX;
    return (uint32_t)(norm * GRID_MAX_UINT);
}

double grid_to_lat(uint32_t grid_y, SpatialzCtx ctx)
{
    const double norm = (double)grid_y / GRID_MAX_UINT;
    return ctx.min_lat + norm * (ctx.max_lat - ctx.min_lat);
}

double grid_to_lon(uint32_t grid_x, SpatialzCtx ctx)
{
    const double norm = (double)grid_x / GRID_MAX_UINT;
    return ctx.min_long + norm * (ctx.max_long - ctx.min_long);
}

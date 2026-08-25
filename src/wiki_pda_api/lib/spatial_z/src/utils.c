#include <stdint.h>
#include <math.h>
#include "../include/spatial_z.h"
#include "utils.h"

void to_internal_sphere(float user_y, float user_x, SpatialzCtx ctx, 
                        float *internal_y, float *internal_x) 
{
    float y = user_y - ctx.min_axis1;
    float x = user_x - ctx.min_axis2;

    y = fmodf(y, 360.0f);
    if (y < 0.0f) y += 360.0f;

    if (y > 180.0f) {
        y = 360.0f - y;
        x += 180.0f;
    }

    x = fmodf(x, 360.0f);
    if (x < 0.0f) x += 360.0f;

    *internal_y = y;
    *internal_x = x;
}

void to_user_space(float internal_y, float internal_x, SpatialzCtx ctx, 
                   float *user_y, float *user_x) 
{
    *user_y = internal_y + ctx.min_axis1;
    *user_x = internal_x + ctx.min_axis2;
}


uint32_t axis1_to_grid(float internal_y)
{
    const float norm = internal_y / 180.0f;

    if (norm <= 0.0f) return 0U;
    if (norm >= 1.0f) return (uint32_t)GRID_MAX_UINT;

    return (uint32_t)(norm * GRID_MAX_UINT);
}

uint32_t axis2_to_grid(float internal_x)
{
    const float norm = internal_x / 360.0f;

    if (norm <= 0.0f) return 0U;
    if (norm >= 1.0f) return (uint32_t)GRID_MAX_UINT;

    return (uint32_t)(norm * GRID_MAX_UINT);
}


float grid_to_axis1(uint32_t grid_y)
{
    const float norm = (float)grid_y / GRID_MAX_UINT;
    return norm * 180.0f;
}

float grid_to_axis2(uint32_t grid_x)
{
    const float norm = (float)grid_x / GRID_MAX_UINT;
    return norm * 360.0f;
}

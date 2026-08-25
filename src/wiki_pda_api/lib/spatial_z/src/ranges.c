#include "../include/spatial_z.h"
#include "utils.h"

#include <float.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

typedef struct {
    float center_axis1;
    float center_axis2;

    float radius;
    float radius_sq;
    float radius_chord_sq;

    float units_per_degree_axis1;
    float units_per_degree_axis2;

    float center_axis1_rad;
    float center_axis2_rad;
    float radius_rad;
    float sphere_radius;
} FastQueryCtx;

typedef enum {
    BLOCK_OUTSIDE = 0,
    BLOCK_INSIDE = 1,
    BLOCK_INTERSECT = 2
} BlockClass;

typedef struct {
    uint32_t gx;
    uint32_t gy;

    uint8_t level;
    uint8_t query_id;

    BlockClass cls;
    float dead_area;
} ZBlock;

static inline float deg_to_rad(float value) {
    return value * (SPATIALZ_PI / 180.0f);
}

static inline float rad_to_deg(float value) {
    return value * (180.0f / SPATIALZ_PI);
}

static inline float clamp_float(float value, float minimum, float maximum) {
    if (value < minimum) return minimum;
    if (value > maximum) return maximum;
    return value;
}

static inline void latlon_to_vector(float lat_rad, float lon_rad, float out_v[3]) {
    const float cos_lat = cosf(lat_rad);
    out_v[0] = cos_lat * cosf(lon_rad);
    out_v[1] = cos_lat * sinf(lon_rad);
    out_v[2] = sinf(lat_rad);
}

static inline float chord_distance_sq(const float v1[3], const float v2[3]) {
    const float dx = v1[0] - v2[0];
    const float dy = v1[1] - v2[1];
    const float dz = v1[2] - v2[2];
    return dx * dx + dy * dy + dz * dz;
}

static inline uint64_t morton_encode_grid(uint32_t gx, uint32_t gy) {
    return spread_bits_32_to_64(gx) | (spread_bits_32_to_64(gy) << 1);
}

static int compare_ranges(const void *a, const void *b) {
    const MortonRange *ra = (const MortonRange *)a;
    const MortonRange *rb = (const MortonRange *)b;

    if (ra->start_code < rb->start_code) return -1;
    if (ra->start_code > rb->start_code) return 1;
    if (ra->end_code < rb->end_code) return -1;
    if (ra->end_code > rb->end_code) return 1;

    return 0;
}

static int merge_ranges(MortonRange *ranges, int count) {
    if (count <= 1) return count;

    qsort(ranges, (size_t)count, sizeof(*ranges), compare_ranges);

    int write = 0;
    for (int read = 1; read < count; ++read) {
        MortonRange *current = &ranges[write];
        const MortonRange *next = &ranges[read];

        const bool overlaps = next->start_code <= current->end_code;
        const bool adjacent = current->end_code != UINT64_MAX &&
                              next->start_code == current->end_code + 1ULL;

        if (overlaps || adjacent) {
            if (next->end_code > current->end_code) {
                current->end_code = next->end_code;
            }
        } else {
            ++write;
            ranges[write] = *next;
        }
    }
    return write + 1;
}

static inline float angular_distance(float axis1_a_rad, float axis2_a_rad, float axis1_b_rad, float axis2_b_rad) {
    const float d_axis1 = axis1_b_rad - axis1_a_rad;
    const float d_axis2 = axis2_b_rad - axis2_a_rad;

    const float s1 = sinf(d_axis1 * 0.5f);
    const float s2 = sinf(d_axis2 * 0.5f);

    const float a = s1 * s1 + cosf(axis1_a_rad) * cosf(axis1_b_rad) * s2 * s2;
    return 2.0f * asinf(sqrtf(clamp_float(a, 0.0f, 1.0f)));
}

static float block_area_estimate(float min_axis1, float max_axis1, float min_axis2, float max_axis2, const FastQueryCtx *query) {
    const float height = fabsf(max_axis1 - min_axis1) * query->units_per_degree_axis1;
    const float mid_axis1 = 0.5f * (min_axis1 + max_axis1);
    const float axis2_scale = fabsf(cosf(deg_to_rad(mid_axis1 - 90.0f)));
    const float width = fabsf(max_axis2 - min_axis2) * query->units_per_degree_axis1 * axis2_scale;
    return height * width;
}

static void get_block_bounds(const ZBlock *block, float *min_axis1, float *max_axis1, float *min_axis2, float *max_axis2) {
    const uint64_t size = 1ULL << block->level;
    const float axis1_0 = (float)grid_to_axis1(block->gy);
    const float axis2_0 = (float)grid_to_axis2(block->gx);

    float axis1_1, axis2_1;

    if (block->level == SPATIALZ_MAX_GRID_LEVEL) {
        axis1_1 = 180.0f;
        axis2_1 = 360.0f;
    } else {
        const uint64_t gy1 = (uint64_t)block->gy + size;
        const uint64_t gx1 = (uint64_t)block->gx + size;
        axis1_1 = ((float)gy1 / GRID_MAX_UINT) * 180.0f;
        axis2_1 = ((float)gx1 / GRID_MAX_UINT) * 360.0f;
    }

    *min_axis1 = fminf(axis1_0, axis1_1);
    *max_axis1 = fmaxf(axis1_0, axis1_1);
    *min_axis2 = fminf(axis2_0, axis2_1);
    *max_axis2 = fmaxf(axis2_0, axis2_1);
}

static BlockClass classify_spherical(ZBlock *block, const FastQueryCtx *query) {
    float min_axis1, max_axis1, min_axis2, max_axis2;
    get_block_bounds(block, &min_axis1, &max_axis1, &min_axis2, &max_axis2);

    const float mid_axis1 = 0.5f * (min_axis1 + max_axis1);
    const float mid_axis2 = 0.5f * (min_axis2 + max_axis2);

    float center_v[3];
    latlon_to_vector(deg_to_rad(mid_axis1 - 90.0f), deg_to_rad(mid_axis2), center_v);

    float query_center_v[3];
    latlon_to_vector(deg_to_rad(query->center_axis1 - 90.0f), deg_to_rad(query->center_axis2), query_center_v);

    const float center_dist_sq = chord_distance_sq(query_center_v, center_v);

    float corner_v[3];
    float max_corner_chord_sq = 0.0f;

    latlon_to_vector(deg_to_rad(min_axis1 - 90.0f), deg_to_rad(min_axis2), corner_v);
    max_corner_chord_sq = fmaxf(max_corner_chord_sq, chord_distance_sq(center_v, corner_v));
    latlon_to_vector(deg_to_rad(max_axis1 - 90.0f), deg_to_rad(min_axis2), corner_v);
    max_corner_chord_sq = fmaxf(max_corner_chord_sq, chord_distance_sq(center_v, corner_v));
    latlon_to_vector(deg_to_rad(min_axis1 - 90.0f), deg_to_rad(max_axis2), corner_v);
    max_corner_chord_sq = fmaxf(max_corner_chord_sq, chord_distance_sq(center_v, corner_v));
    latlon_to_vector(deg_to_rad(max_axis1 - 90.0f), deg_to_rad(max_axis2), corner_v);
    max_corner_chord_sq = fmaxf(max_corner_chord_sq, chord_distance_sq(center_v, corner_v));

    const float block_chord_radius = sqrtf(max_corner_chord_sq) * 1.001f;
    const float query_radius = sqrtf(query->radius_chord_sq);
    const float center_dist = sqrtf(center_dist_sq);

    if (center_dist - block_chord_radius > query_radius) {
        block->dead_area = block_area_estimate(min_axis1, max_axis1, min_axis2, max_axis2, query);
        return BLOCK_OUTSIDE;
    }

    if (center_dist + block_chord_radius <= query_radius) {
        block->dead_area = 0.0f;
        return BLOCK_INSIDE;
    }

    float sin_a1[3], cos_a1[3], sin_a2[3], cos_a2[3];
    const float axis1_samples[3] = { min_axis1, mid_axis1, max_axis1 };
    const float axis2_samples[3] = { min_axis2, mid_axis2, max_axis2 };

    for (int i = 0; i < 3; ++i) {
        sin_a1[i] = sinf(deg_to_rad(axis1_samples[i] - 90.0f));
        cos_a1[i] = cosf(deg_to_rad(axis1_samples[i] - 90.0f));
        sin_a2[i] = sinf(deg_to_rad(axis2_samples[i]));
        cos_a2[i] = cosf(deg_to_rad(axis2_samples[i]));
    }

    int inside = 0;
    for (int i = 0; i < 3; ++i) {
        for (int j = 0; j < 3; ++j) {
            float sample_v[3];
            sample_v[0] = cos_a1[i] * cos_a2[j];
            sample_v[1] = cos_a1[i] * sin_a2[j];
            sample_v[2] = sin_a1[i];

            if (chord_distance_sq(query_center_v, sample_v) <= query->radius_chord_sq) {
                ++inside;
            }
        }
    }

    const float area = block_area_estimate(min_axis1, max_axis1, min_axis2, max_axis2, query);
    block->dead_area = area * (1.0f - ((float)inside / 9.0f));
    return BLOCK_INTERSECT;
}

static void get_grid_box(float min_axis2, float max_axis2, float min_axis1, float max_axis1, uint32_t *min_gx, uint32_t *max_gx, uint32_t *min_gy, uint32_t *max_gy) {
    const uint32_t gx0 = axis2_to_grid(min_axis2);
    const uint32_t gx1 = axis2_to_grid(max_axis2);
    const uint32_t gy0 = axis1_to_grid(min_axis1);
    const uint32_t gy1 = axis1_to_grid(max_axis1);

    *min_gx = gx0 < gx1 ? gx0 : gx1;
    *max_gx = gx0 > gx1 ? gx0 : gx1;
    *min_gy = gy0 < gy1 ? gy0 : gy1;
    *max_gy = gy0 > gy1 ? gy0 : gy1;
}

static uint64_t next_pow2_u64(uint64_t value) {
    if (value <= 1ULL) return 1ULL;
    --value;
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    value |= value >> 32;
    return value + 1ULL;
}

static uint64_t count_root_blocks(uint32_t min_gx, uint32_t max_gx, uint32_t min_gy, uint32_t max_gy, uint64_t size) {
    if (size == 0ULL) return UINT64_MAX;
    const uint64_t mask = ~(size - 1ULL);
    const uint64_t start_x = min_gx & mask;
    const uint64_t end_x = max_gx & mask;
    const uint64_t start_y = min_gy & mask;
    const uint64_t end_y = max_gy & mask;

    const uint64_t nx = ((end_x - start_x) >> __builtin_ctzll(size)) + 1ULL;
    const uint64_t ny = ((end_y - start_y) >> __builtin_ctzll(size)) + 1ULL;

    if (nx > UINT64_MAX / ny) return UINT64_MAX;
    return nx * ny;
}

static uint8_t level_from_size(uint64_t size) {
    uint8_t level = 0;
    while (level < SPATIALZ_MAX_GRID_LEVEL && (1ULL << level) < size) {
        ++level;
    }
    return level;
}

static bool get_box_grid_info(float min_axis2, float max_axis2, float min_axis1, float max_axis1, uint32_t *min_gx, uint32_t *max_gx, uint32_t *min_gy, uint32_t *max_gy, uint8_t *minimum_level) {
    get_grid_box(min_axis2, max_axis2, min_axis1, max_axis1, min_gx, max_gx, min_gy, max_gy);
    const uint64_t span_x = (uint64_t)*max_gx - *min_gx + 1ULL;
    const uint64_t span_y = (uint64_t)*max_gy - *min_gy + 1ULL;
    const uint64_t span = span_x > span_y ? span_x : span_y;

    uint64_t size = next_pow2_u64(span);
    if (size > (1ULL << SPATIALZ_MAX_GRID_LEVEL)) size = 1ULL << SPATIALZ_MAX_GRID_LEVEL;

    *minimum_level = level_from_size(size);
    return true;
}

static int create_root_blocks(float min_axis2, float max_axis2, float min_axis1, float max_axis1, uint8_t query_id, uint8_t level, int max_ranges, const FastQueryCtx *queries, ZBlock *blocks) {
    uint32_t min_gx, max_gx, min_gy, max_gy;
    uint8_t minimum_level;

    if (!get_box_grid_info(min_axis2, max_axis2, min_axis1, max_axis1, &min_gx, &max_gx, &min_gy, &max_gy, &minimum_level)) return -1;
    if (level < minimum_level) level = minimum_level;

    const uint64_t size = 1ULL << level;
    const uint64_t mask = ~(size - 1ULL);
    const uint64_t start_x = min_gx & mask;
    const uint64_t end_x = max_gx & mask;
    const uint64_t start_y = min_gy & mask;
    const uint64_t end_y = max_gy & mask;

    if (count_root_blocks(min_gx, max_gx, min_gy, max_gy, size) > (uint64_t)max_ranges) return -1;

    int count = 0;
    for (uint64_t gy = start_y; gy <= end_y;) {
        for (uint64_t gx = start_x; gx <= end_x;) {
            if (count >= max_ranges) return -1;

            ZBlock block = { .gx = (uint32_t)gx, .gy = (uint32_t)gy, .level = level, .query_id = query_id, .dead_area = 0.0f };
            block.cls = classify_spherical(&block, &queries[query_id]);

            if (block.cls != BLOCK_OUTSIDE) blocks[count++] = block;

            if (gx + size < gx || gx + size > end_x) break;
            gx += size;
        }
        if (gy + size < gy || gy + size > end_y) break;
        gy += size;
    }
    return count;
}

static int make_children(const ZBlock *parent, const FastQueryCtx *queries, ZBlock children[4]) {
    if (parent->level == 0) return 0;

    const uint8_t child_level = (uint8_t)(parent->level - 1U);
    const uint64_t child_size = 1ULL << child_level;
    int count = 0;

    for (int iy = 0; iy < 2; ++iy) {
        for (int ix = 0; ix < 2; ++ix) {
            ZBlock child = {
                .gx = (uint32_t)(parent->gx + (ix ? child_size : 0ULL)),
                .gy = (uint32_t)(parent->gy + (iy ? child_size : 0ULL)),
                .level = child_level,
                .query_id = parent->query_id,
                .dead_area = 0.0f
            };
            child.cls = classify_spherical(&child, &queries[parent->query_id]);
            if (child.cls != BLOCK_OUTSIDE) children[count++] = child;
        }
    }
    return count;
}

static MortonRange encode_block(const ZBlock *block)
{
    MortonRange range;

    if (block->level == SPATIALZ_MAX_GRID_LEVEL) {
        range.start_code = 0ULL;
        if (SPATIALZ_MAX_GRID_LEVEL == 32) {
            range.end_code = UINT64_MAX;
        } else {
            range.end_code = (1ULL << (2 * SPATIALZ_MAX_GRID_LEVEL)) - 1ULL;
        }
        return range;
    }

    const uint64_t start = morton_encode_grid(block->gx, block->gy);
    const uint32_t bit_count = 2U * block->level;
    const uint64_t count = 1ULL << bit_count;

    range.start_code = start;
    range.end_code = start + count - 1ULL;

    return range;
}

static void refine_blocks(ZBlock *blocks, int *count, int max_ranges, const FastQueryCtx *queries) {
    for (;;) {
        int best_index = -1;
        float largest_dead_area = -1.0f;

        for (int i = 0; i < *count; ++i) {
            if (blocks[i].cls == BLOCK_INTERSECT && blocks[i].level > 0) {
                if (blocks[i].dead_area > largest_dead_area) {
                    largest_dead_area = blocks[i].dead_area;
                    best_index = i;
                }
            }
        }

        if (best_index < 0) break;

        ZBlock children[4];
        const int child_count = make_children(&blocks[best_index], queries, children);

        if (*count - 1 + child_count > max_ranges) break;

        if (child_count == 0) {
            blocks[best_index] = blocks[*count - 1];
            (*count)--;
        } else {
            blocks[best_index] = children[0];
            for (int c = 1; c < child_count; ++c) {
                blocks[*count] = children[c];
                (*count)++;
            }
        }
    }
}

static bool make_query(float center_axis1, float center_axis2, float radius, const SpatialzCtx *ctx, FastQueryCtx *query) {
    if (!isfinite(radius) || radius < 0.0f || ctx->unit_length <= 0.0f) return false;

    float int_y_dbl, int_x_dbl;
    to_internal_sphere(center_axis1, center_axis2, *ctx, &int_y_dbl, &int_x_dbl);

    query->center_axis1 = (float)int_y_dbl;
    query->center_axis2 = (float)int_x_dbl;
    query->radius = radius;
    query->radius_sq = radius * radius;

    query->center_axis1_rad = deg_to_rad(query->center_axis1 - 90.0f);
    query->center_axis2_rad = deg_to_rad(query->center_axis2);
    query->sphere_radius = (float)ctx->unit_length * (180.0f / SPATIALZ_PI);

    const float axis2_scale = fabsf(cosf(query->center_axis1_rad));
    query->units_per_degree_axis1 = (float)ctx->unit_length;
    query->units_per_degree_axis2 = (float)ctx->unit_length * axis2_scale;

    query->radius_rad = query->sphere_radius > 0.0f ? radius / query->sphere_radius : 0.0f;

    if (query->radius_rad > SPATIALZ_PI) {
        query->radius_rad = SPATIALZ_PI;
    }

    const float half_chord = sinf(query->radius_rad * 0.5f);
    query->radius_chord_sq = 4.0f * half_chord * half_chord;

    return true;
}

static void get_query_segments(const FastQueryCtx *query, float *min_axis1, float *max_axis1, float *segment1_min_axis2, float *segment1_max_axis2, float *segment2_min_axis2, float *segment2_max_axis2, int *segment_count, FastQueryCtx queries[2]) {
    *segment_count = 0;
    queries[0] = *query;
    queries[1] = *query;

    const float MIN_AXIS1 = 0.0f;
    const float MAX_AXIS1 = 180.0f;
    const float MIN_AXIS2 = 0.0f;
    const float MAX_AXIS2 = 360.0f;
    const float period = 360.0f;

    const float alpha = query->radius_rad;
    const float axis1_center = query->center_axis1_rad;

    if (alpha >= SPATIALZ_PI) {
        *min_axis1 = MIN_AXIS1; *max_axis1 = MAX_AXIS1;
        *segment1_min_axis2 = MIN_AXIS2; *segment1_max_axis2 = MAX_AXIS2;
        *segment_count = 1;
        return;
    }

    *min_axis1 = rad_to_deg(fmaxf(-SPATIALZ_PI * 0.5f, axis1_center - alpha)) + 90.0f;
    *max_axis1 = rad_to_deg(fminf(SPATIALZ_PI * 0.5f, axis1_center + alpha)) + 90.0f;

    if (axis1_center + alpha >= SPATIALZ_PI * 0.5f || axis1_center - alpha <= -SPATIALZ_PI * 0.5f || fabsf(cosf(axis1_center)) <= 1e-15f) {
        *segment1_min_axis2 = MIN_AXIS2; *segment1_max_axis2 = MAX_AXIS2;
        *segment_count = 1;
        return;
    }

    const float ratio = clamp_float(sinf(alpha) / fabsf(cosf(axis1_center)), -1.0f, 1.0f);
    const float d_axis2_deg = rad_to_deg(asinf(fabsf(ratio)));

    if (d_axis2_deg >= period * 0.5f) {
        *segment1_min_axis2 = MIN_AXIS2; *segment1_max_axis2 = MAX_AXIS2;
        *segment_count = 1;
        return;
    }

    const float left = query->center_axis2 - d_axis2_deg;
    const float right = query->center_axis2 + d_axis2_deg;

    if (left >= MIN_AXIS2 && right <= MAX_AXIS2) {
        *segment1_min_axis2 = left; *segment1_max_axis2 = right;
        *segment_count = 1;
        return;
    }

    if (left < MIN_AXIS2) {
        *segment1_min_axis2 = MIN_AXIS2; *segment1_max_axis2 = right;
        *segment2_min_axis2 = MAX_AXIS2 - (MIN_AXIS2 - left); *segment2_max_axis2 = MAX_AXIS2;
        queries[1].center_axis2 = query->center_axis2 + period;
        *segment_count = 2;
        return;
    }

    *segment1_min_axis2 = left; *segment1_max_axis2 = MAX_AXIS2;
    *segment2_min_axis2 = MIN_AXIS2; *segment2_max_axis2 = MIN_AXIS2 + (right - MAX_AXIS2);
    queries[1].center_axis2 = query->center_axis2 - period;
    *segment_count = 2;
}

static bool determine_root_level(float segment1_min_axis2, float segment1_max_axis2, float segment2_min_axis2, float segment2_max_axis2, int segment_count, float min_axis1, float max_axis1, int max_ranges, uint8_t *root_level) {
    uint32_t min_gx[2], max_gx[2], min_gy[2], max_gy[2];
    uint8_t minimum_level[2];

    float segment_min_axis2[2] = { segment1_min_axis2, segment2_min_axis2 };
    float segment_max_axis2[2] = { segment1_max_axis2, segment2_max_axis2 };

    uint8_t level = 0;

    for (int i = 0; i < segment_count; ++i) {
        if (!get_box_grid_info(segment_min_axis2[i], segment_max_axis2[i], min_axis1, max_axis1, &min_gx[i], &max_gx[i], &min_gy[i], &max_gy[i], &minimum_level[i])) return false;
        if (minimum_level[i] > level) level = minimum_level[i];
    }

    for (;;) {
        const uint64_t size = 1ULL << level;
        uint64_t total_blocks = 0;

        for (int i = 0; i < segment_count; ++i) {
            const uint64_t count = count_root_blocks(min_gx[i], max_gx[i], min_gy[i], max_gy[i], size);
            if (count > UINT64_MAX - total_blocks) total_blocks = UINT64_MAX;
            else total_blocks += count;
        }

        if (total_blocks <= (uint64_t)max_ranges) {
            *root_level = level;
            return true;
        }

        if (level >= SPATIALZ_MAX_GRID_LEVEL) return false;
        ++level;
    }
}

bool spatial_get_radius_ranges(
    float center_axis1,
    float center_axis2,
    float radius,
    MortonRange *out_ranges,
    int *out_num_ranges,
    int max_ranges,
    const SpatialzCtx *ctx)
{
    if (!out_ranges || !out_num_ranges || max_ranges <= 0) return false;

    #if defined(DEBUG_MODE)
    printf("[SPATIALZ DEBUG] spatial_get_radius_ranges invoked:\n");
    printf("  -> center_axis1: %.6f\n", center_axis1);
    printf("  -> center_axis2: %.6f\n", center_axis2);
    printf("  -> radius:       %.6f\n", radius);
    printf("  -> max_ranges:   %d\n", max_ranges);
    if (ctx) {
        printf("  -> ctx: min_axis1=%.2f, min_axis2=%.2f, unit_length=%.2f\n", 
               ctx->min_axis1, ctx->min_axis2, ctx->unit_length);
    } else {
        printf("  -> ctx: NULL\n");
    }
    #endif

    if (radius <= 0.0f) {
        float int_y_dbl, int_x_dbl;
        to_internal_sphere(center_axis1, center_axis2, *ctx, &int_y_dbl, &int_x_dbl);

        uint32_t gx = axis2_to_grid((float)int_x_dbl);
        uint32_t gy = axis1_to_grid((float)int_y_dbl - 90.0f);

        uint64_t code = morton_encode_grid(gx, gy);

        out_ranges[0].start_code = code;
        out_ranges[0].end_code = code;
        *out_num_ranges = 1;
        return true;
    }

    if (max_ranges > SPATIALZ_MAX_INTERNAL_BLOCKS) {
        max_ranges = SPATIALZ_MAX_INTERNAL_BLOCKS;
    }

    *out_num_ranges = 0;

    ZBlock blocks[SPATIALZ_MAX_INTERNAL_BLOCKS];

    FastQueryCtx query;
    if (!make_query(center_axis1, center_axis2, radius, ctx, &query)) return false;

    FastQueryCtx queries[2];
    float min_axis1, max_axis1, segment1_min_axis2, segment1_max_axis2;
    float segment2_min_axis2 = 0.0f, segment2_max_axis2 = 0.0f;
    int segment_count = 0;

    get_query_segments(
        &query, &min_axis1, &max_axis1, 
        &segment1_min_axis2, &segment1_max_axis2, 
        &segment2_min_axis2, &segment2_max_axis2, 
        &segment_count, queries
    );

    uint8_t root_level = 0;
    if (!determine_root_level(
            segment1_min_axis2, segment1_max_axis2, 
            segment2_min_axis2, segment2_max_axis2, 
            segment_count, min_axis1, max_axis1, 
            max_ranges, &root_level)) {
        return false;
    }

    int block_count = create_root_blocks(
        segment1_min_axis2, segment1_max_axis2, 
        min_axis1, max_axis1, 0, root_level, max_ranges, 
        queries, blocks
    );
    if (block_count < 0) return false;

    if (segment_count == 2) {
        const int remaining = max_ranges - block_count;
        if (remaining <= 0) return false;

        const int second_count = create_root_blocks(
            segment2_min_axis2, segment2_max_axis2, 
            min_axis1, max_axis1, 1, root_level, remaining, 
            queries, blocks + block_count 
        );
        if (second_count < 0) return false;
        block_count += second_count;
    }

    if (block_count <= 0) return false;

    refine_blocks(blocks, &block_count, max_ranges, queries);

    int range_count = 0;
    for (int i = 0; i < block_count; ++i) {
        if (blocks[i].cls == BLOCK_OUTSIDE) continue;
        out_ranges[range_count++] = encode_block(&blocks[i]);
    }

    range_count = merge_ranges(out_ranges, range_count);

    if (range_count <= 0 || range_count > max_ranges) {
        *out_num_ranges = 0;
        return false;
    }

    *out_num_ranges = range_count;
    return true;
}

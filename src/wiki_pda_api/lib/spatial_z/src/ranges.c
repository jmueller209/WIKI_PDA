#include "../include/spatial_z.h"
#include "utils.h"

#include <float.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#ifndef SPATIALZ_FAST_RADIUS_KM
#define SPATIALZ_FAST_RADIUS_KM 1000.0
#endif

#ifndef SPATIALZ_SPHERICAL_LAT_THRESHOLD_DEG
#define SPATIALZ_SPHERICAL_LAT_THRESHOLD_DEG 75.0
#endif

#define SPATIALZ_MAX_GRID_LEVEL 32U
#define SPATIALZ_EPS_KM 1e-8

typedef enum {
    QUERY_LOCAL = 0,
    QUERY_SPHERICAL = 1
} QueryMode;

typedef struct {
    double center_lat;
    double center_lon;
    double radius_km;
    double radius_sq_km;
    QueryMode mode;

    // Used by the local/equirectangular fast path.
    double km_per_deg_lat;
    double km_per_deg_lon;

    // Used by the spherical path.
    double center_lat_rad;
    double center_lon_rad;
    double radius_rad;
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
    double dead_area;
} ZBlock;


static inline uint64_t morton_encode_grid(uint32_t gx, uint32_t gy)
{
    return spread_bits_32_to_64(gx) |
           (spread_bits_32_to_64(gy) << 1);
}

static int compare_ranges(const void *a, const void *b)
{
    const SpatialRange *ra = (const SpatialRange *)a;
    const SpatialRange *rb = (const SpatialRange *)b;

    if (ra->start_code < rb->start_code) return -1;
    if (ra->start_code > rb->start_code) return 1;
    if (ra->end_code < rb->end_code) return -1;
    if (ra->end_code > rb->end_code) return 1;
    return 0;
}

static int merge_ranges(SpatialRange *ranges, int count)
{
    if (count <= 1)
        return count;

    qsort(ranges, (size_t)count, sizeof(*ranges), compare_ranges);

    int write = 0;

    for (int read = 1; read < count; ++read) {
        SpatialRange *cur = &ranges[write];
        const SpatialRange *next = &ranges[read];

        const bool overlaps = next->start_code <= cur->end_code;
        const bool adjacent =
            cur->end_code != UINT64_MAX &&
            next->start_code == cur->end_code + 1ULL;

        if (overlaps || adjacent) {
            if (next->end_code > cur->end_code)
                cur->end_code = next->end_code;
        } else {
            ++write;
            ranges[write] = *next;
        }
    }

    return write + 1;
}


static inline double deg_to_rad(double x)
{
    return x * (M_PI / 180.0);
}

static inline double rad_to_deg(double x)
{
    return x * (180.0 / M_PI);
}

static inline double clamp_double(double x, double lo, double hi)
{
    return x < lo ? lo : (x > hi ? hi : x);
}

static inline double normalize_lon_deg(double lon)
{
    while (lon < -180.0) lon += 360.0;
    while (lon > 180.0) lon -= 360.0;
    return lon;
}

static inline double earth_radius_km(const SpatialzCtx *ctx)
{
    return ctx->unit_length * (180.0 / M_PI);
}

static inline double local_sq_dist(double lat, double lon, const FastQueryCtx *q)
{
    const double dy = (lat - q->center_lat) * q->km_per_deg_lat;
    const double dx = (lon - q->center_lon) * q->km_per_deg_lon;
    return dx * dx + dy * dy;
}

static inline double spherical_angle(
    double lat1_rad,
    double lon1_rad,
    double lat2_rad,
    double lon2_rad)
{
    const double dlat = lat2_rad - lat1_rad;
    const double dlon = lon2_rad - lon1_rad;

    const double s1 = sin(dlat * 0.5);
    const double s2 = sin(dlon * 0.5);
    const double a = s1 * s1 +
                     cos(lat1_rad) * cos(lat2_rad) * s2 * s2;

    return 2.0 * asin(sqrt(clamp_double(a, 0.0, 1.0)));
}

static inline double spherical_distance_km(
    double lat,
    double lon,
    const FastQueryCtx *q,
    double earth_radius_km)
{
    return spherical_angle(
        q->center_lat_rad,
        q->center_lon_rad,
        deg_to_rad(lat),
        deg_to_rad(lon)) * earth_radius_km;
}

static double block_dead_area_estimate(
    double min_lat,
    double max_lat,
    double min_lon,
    double max_lon,
    const FastQueryCtx *q)
{
    const double h = fabs(max_lat - min_lat) * q->km_per_deg_lat;
    const double w = fabs(max_lon - min_lon) * q->km_per_deg_lon;
    return w * h;
}

static void get_block_bounds(
    const ZBlock *b,
    SpatialzCtx ctx,
    double *min_lat,
    double *max_lat,
    double *min_lon,
    double *max_lon)
{
    const uint64_t size = 1ULL << b->level;

    const uint64_t gx1_64 = (uint64_t)b->gx + size - 1ULL;
    const uint64_t gy1_64 = (uint64_t)b->gy + size - 1ULL;

    const uint32_t gx1 = (uint32_t)gx1_64;
    const uint32_t gy1 = (uint32_t)gy1_64;

    const double lat0 = grid_to_lat(b->gy, ctx);
    const double lat1 = grid_to_lat(gy1, ctx);
    const double lon0 = grid_to_lon(b->gx, ctx);
    const double lon1 = grid_to_lon(gx1, ctx);

    *min_lat = fmin(lat0, lat1);
    *max_lat = fmax(lat0, lat1);
    *min_lon = fmin(lon0, lon1);
    *max_lon = fmax(lon0, lon1);
}


static BlockClass classify_local(
    ZBlock *b,
    const FastQueryCtx *q,
    SpatialzCtx ctx)
{
    double min_lat, max_lat, min_lon, max_lon;
    get_block_bounds(b, ctx, &min_lat, &max_lat, &min_lon, &max_lon);

    const double closest_lat = clamp_double(
        q->center_lat, min_lat, max_lat);
    const double closest_lon = clamp_double(
        q->center_lon, min_lon, max_lon);

    const double min_d2 = local_sq_dist(closest_lat, closest_lon, q);

    if (min_d2 > q->radius_sq_km) {
        b->dead_area = block_dead_area_estimate(
            min_lat, max_lat, min_lon, max_lon, q);
        return BLOCK_OUTSIDE;
    }

    const double d00 = local_sq_dist(min_lat, min_lon, q);
    const double d01 = local_sq_dist(min_lat, max_lon, q);
    const double d10 = local_sq_dist(max_lat, min_lon, q);
    const double d11 = local_sq_dist(max_lat, max_lon, q);

    const double max_d2 = fmax(fmax(d00, d01), fmax(d10, d11));

    if (max_d2 <= q->radius_sq_km)
        return BLOCK_INSIDE;

    const double lat_mid = 0.5 * (min_lat + max_lat);
    const double lon_mid = 0.5 * (min_lon + max_lon);
    const double lat[3] = { min_lat, lat_mid, max_lat };
    const double lon[3] = { min_lon, lon_mid, max_lon };

    int inside = 0;
    for (int iy = 0; iy < 3; ++iy) {
        for (int ix = 0; ix < 3; ++ix) {
            if (local_sq_dist(lat[iy], lon[ix], q) <= q->radius_sq_km)
                ++inside;
        }
    }

    const double area = block_dead_area_estimate(
        min_lat, max_lat, min_lon, max_lon, q);
    b->dead_area = area * (1.0 - ((double)inside / 9.0));

    return BLOCK_INTERSECT;
}

static BlockClass classify_spherical(
    ZBlock *b,
    const FastQueryCtx *q,
    SpatialzCtx ctx)
{
    double min_lat, max_lat, min_lon, max_lon;
    get_block_bounds(b, ctx, &min_lat, &max_lat, &min_lon, &max_lon);

    const double lat_mid = 0.5 * (min_lat + max_lat);
    const double lon_mid = 0.5 * (min_lon + max_lon);

    const double lat_half = 0.5 * deg_to_rad(max_lat - min_lat);
    const double lon_half = 0.5 * deg_to_rad(max_lon - min_lon);

    double block_radius = lat_half + lon_half;
    if (block_radius > M_PI)
        block_radius = M_PI;

    const double center_angle = spherical_angle(
        q->center_lat_rad,
        q->center_lon_rad,
        deg_to_rad(lat_mid),
        deg_to_rad(lon_mid));

    const double padded_radius = q->radius_rad + deg_to_rad(1e-8);

    if (center_angle - block_radius > padded_radius) {
        b->dead_area = block_dead_area_estimate(
            min_lat, max_lat, min_lon, max_lon, q);
        return BLOCK_OUTSIDE;
    }

    if (center_angle + block_radius <= padded_radius) {
        b->dead_area = 0.0;
        return BLOCK_INSIDE;
    }

    const double R = earth_radius_km(&ctx);
    const double lat_span_km = fabs(max_lat - min_lat) * ctx.unit_length;
    const double lon_scale = fmax(0.0, cos(deg_to_rad(lat_mid)));
    const double lon_span_km =
        fabs(max_lon - min_lon) * ctx.unit_length * lon_scale;

    const double area = lat_span_km * lon_span_km;

    const double lat[3] = { min_lat, lat_mid, max_lat };
    const double lon[3] = { min_lon, lon_mid, max_lon };
    int inside = 0;

    for (int iy = 0; iy < 3; ++iy) {
        for (int ix = 0; ix < 3; ++ix) {
            if (spherical_distance_km(lat[iy], lon[ix], q, R) <= q->radius_km)
                ++inside;
        }
    }

    b->dead_area = area * (1.0 - ((double)inside / 9.0));
    return BLOCK_INTERSECT;
}

static BlockClass classify_block(
    ZBlock *b,
    const FastQueryCtx *q,
    SpatialzCtx ctx)
{
    if (q->mode == QUERY_SPHERICAL)
        return classify_spherical(b, q, ctx);
    return classify_local(b, q, ctx);
}

static void get_grid_box(
    double min_lon,
    double max_lon,
    double min_lat,
    double max_lat,
    SpatialzCtx ctx,
    uint32_t *min_gx,
    uint32_t *max_gx,
    uint32_t *min_gy,
    uint32_t *max_gy)
{
    uint32_t gx0 = lon_to_grid(min_lon, ctx);
    uint32_t gx1 = lon_to_grid(max_lon, ctx);
    uint32_t gy0 = lat_to_grid(min_lat, ctx);
    uint32_t gy1 = lat_to_grid(max_lat, ctx);

    *min_gx = gx0 < gx1 ? gx0 : gx1;
    *max_gx = gx0 > gx1 ? gx0 : gx1;
    *min_gy = gy0 < gy1 ? gy0 : gy1;
    *max_gy = gy0 > gy1 ? gy0 : gy1;
}

static uint64_t next_pow2_u64(uint64_t x)
{
    if (x <= 1ULL)
        return 1ULL;

    --x;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x |= x >> 32;
    return x + 1ULL;
}

static uint64_t count_root_blocks(
    uint32_t min_gx,
    uint32_t max_gx,
    uint32_t min_gy,
    uint32_t max_gy,
    uint64_t size)
{
    if (size == 0ULL)
        return UINT64_MAX;

    const uint64_t x0 = ((uint64_t)min_gx / size) * size;
    const uint64_t x1 = ((uint64_t)max_gx / size) * size;
    const uint64_t y0 = ((uint64_t)min_gy / size) * size;
    const uint64_t y1 = ((uint64_t)max_gy / size) * size;

    const uint64_t nx = ((x1 - x0) / size) + 1ULL;
    const uint64_t ny = ((y1 - y0) / size) + 1ULL;

    if (nx > UINT64_MAX / ny)
        return UINT64_MAX;

    return nx * ny;
}

static int create_root_blocks(
    double min_lon,
    double max_lon,
    double min_lat,
    double max_lat,
    uint8_t query_id,
    int max_ranges,
    SpatialzCtx ctx,
    const FastQueryCtx *queries,
    ZBlock *blocks)
{
    uint32_t min_gx, max_gx, min_gy, max_gy;
    get_grid_box(
        min_lon, max_lon, min_lat, max_lat, ctx,
        &min_gx, &max_gx, &min_gy, &max_gy);

    const uint64_t span_x = (uint64_t)max_gx - min_gx + 1ULL;
    const uint64_t span_y = (uint64_t)max_gy - min_gy + 1ULL;

    uint64_t size = next_pow2_u64(span_x > span_y ? span_x : span_y);
    if (size > (1ULL << SPATIALZ_MAX_GRID_LEVEL))
        size = (1ULL << SPATIALZ_MAX_GRID_LEVEL);

    while (count_root_blocks(
               min_gx, max_gx, min_gy, max_gy, size) > (uint64_t)max_ranges) {
        if (size >= (1ULL << SPATIALZ_MAX_GRID_LEVEL))
            break;
        size <<= 1;
    }

    const uint64_t start_x = ((uint64_t)min_gx / size) * size;
    const uint64_t end_x   = ((uint64_t)max_gx / size) * size;
    const uint64_t start_y = ((uint64_t)min_gy / size) * size;
    const uint64_t end_y   = ((uint64_t)max_gy / size) * size;

    uint8_t level = 0;
    while ((1ULL << level) < size && level < SPATIALZ_MAX_GRID_LEVEL)
        ++level;

    int count = 0;

    for (uint64_t gy = start_y; gy <= end_y; gy += size) {
        for (uint64_t gx = start_x; gx <= end_x; gx += size) {
            if (count >= max_ranges)
                return count;

            ZBlock *b = &blocks[count];
            b->gx = (uint32_t)gx;
            b->gy = (uint32_t)gy;
            b->level = level;
            b->query_id = query_id;
            b->dead_area = 0.0;

            b->cls = classify_block(b, &queries[query_id], ctx);

            if (b->cls != BLOCK_OUTSIDE)
                ++count;

            // Avoid wraparound when size is 2^32.
            if (gx + size > end_x || gx + size < gx)
                break;
        }

        if (gy + size > end_y || gy + size < gy)
            break;
    }

    return count;
}

static int make_children(
    const ZBlock *parent,
    const FastQueryCtx *queries,
    SpatialzCtx ctx,
    ZBlock children[4])
{
    if (parent->level == 0)
        return 0;

    const uint8_t child_level = (uint8_t)(parent->level - 1U);
    const uint64_t child_size = 1ULL << child_level;

    int count = 0;

    for (int iy = 0; iy < 2; ++iy) {
        for (int ix = 0; ix < 2; ++ix) {
            ZBlock child;

            const uint64_t gx =
                (uint64_t)parent->gx + (ix ? child_size : 0ULL);
            const uint64_t gy =
                (uint64_t)parent->gy + (iy ? child_size : 0ULL);

            child.gx = (uint32_t)gx;
            child.gy = (uint32_t)gy;
            child.level = child_level;
            child.query_id = parent->query_id;
            child.dead_area = 0.0;
            child.cls = classify_block(
                &child,
                &queries[parent->query_id],
                ctx);

            if (child.cls != BLOCK_OUTSIDE)
                children[count++] = child;
        }
    }

    return count;
}

static SpatialRange encode_block(const ZBlock *b)
{
    SpatialRange r;

    if (b->level == 32U) {
        r.start_code = 0ULL;
        r.end_code = UINT64_MAX;
        return r;
    }

    const uint64_t start = morton_encode_grid(b->gx, b->gy);
    const uint64_t count = 1ULL << (2U * b->level);

    r.start_code = start;
    r.end_code = start + count - 1ULL;
    return r;
}

static void refine_blocks(
    ZBlock *blocks,
    int *count,
    int max_ranges,
    const FastQueryCtx *queries,
    SpatialzCtx ctx)
{
    for (;;) {
        int best_index = -1;
        double best_score = -DBL_MAX;
        ZBlock best_children[4];
        int best_child_count = 0;

        for (int i = 0; i < *count; ++i) {
            const ZBlock *b = &blocks[i];

            if (b->cls != BLOCK_INTERSECT || b->level == 0)
                continue;

            ZBlock children[4];
            const int child_count = make_children(
                b, queries, ctx, children);

            if (child_count <= 0)
                continue;

            const int new_count = *count - 1 + child_count;
            if (new_count > max_ranges)
                continue;

            double child_dead_area = 0.0;
            for (int c = 0; c < child_count; ++c)
                child_dead_area += children[c].dead_area;

            const double benefit = b->dead_area - child_dead_area;
            if (benefit <= SPATIALZ_EPS_KM)
                continue;

            const int extra_ranges = child_count - 1;
            const double score =
                extra_ranges == 0
                    ? DBL_MAX
                    : benefit / (double)extra_ranges;

            if (score > best_score) {
                best_score = score;
                best_index = i;
                best_child_count = child_count;
                memcpy(best_children, children, sizeof(best_children));
            }
        }

        if (best_index < 0)
            break;

        blocks[best_index] = best_children[0];

        for (int c = 1; c < best_child_count; ++c) {
            if (*count >= max_ranges)
                break;
            blocks[*count] = best_children[c];
            ++(*count);
        }
    }
}


static bool make_query(
    double center_lat,
    double center_lon,
    double radius_km,
    SpatialzCtx ctx,
    FastQueryCtx *q)
{
    if (!isfinite(center_lat) || !isfinite(center_lon) ||
        !isfinite(radius_km) || radius_km < 0.0)
        return false;

    center_lat = clamp_double(center_lat, ctx.min_lat, ctx.max_lat);
    center_lon = normalize_lon_deg(center_lon);

    q->center_lat = center_lat;
    q->center_lon = center_lon;
    q->radius_km = radius_km;
    q->radius_sq_km = radius_km * radius_km;
    q->km_per_deg_lat = ctx.unit_length;
    q->km_per_deg_lon = ctx.unit_length * cos(deg_to_rad(center_lat));
    q->center_lat_rad = deg_to_rad(center_lat);
    q->center_lon_rad = deg_to_rad(center_lon);

    const double R = earth_radius_km(&ctx);
    q->radius_rad = R > 0.0 ? radius_km / R : 0.0;

    q->mode =
        (radius_km <= SPATIALZ_FAST_RADIUS_KM &&
         fabs(center_lat) < SPATIALZ_SPHERICAL_LAT_THRESHOLD_DEG)
            ? QUERY_LOCAL
            : QUERY_SPHERICAL;

    return true;
}

static void get_query_lon_segments(
    const FastQueryCtx *q,
    SpatialzCtx ctx,
    double *min_lat,
    double *max_lat,
    double *seg1_min_lon,
    double *seg1_max_lon,
    double *seg2_min_lon,
    double *seg2_max_lon,
    int *segment_count,
    FastQueryCtx queries[2])
{
    *segment_count = 0;
    queries[0] = *q;
    queries[1] = *q;

    if (q->mode == QUERY_LOCAL) {
        const double dlat = q->radius_km / q->km_per_deg_lat;
        const double min_la = fmax(ctx.min_lat, q->center_lat - dlat);
        const double max_la = fmin(ctx.max_lat, q->center_lat + dlat);
        *min_lat = min_la;
        *max_lat = max_la;

        const double lon_scale = fabs(q->km_per_deg_lon);
        if (lon_scale < 1e-12) {
            *seg1_min_lon = ctx.min_long;
            *seg1_max_lon = ctx.max_long;
            *segment_count = 1;
            return;
        }

        const double dlon = q->radius_km / lon_scale;
        if (dlon >= 180.0) {
            *seg1_min_lon = ctx.min_long;
            *seg1_max_lon = ctx.max_long;
            *segment_count = 1;
            return;
        }

        const double left = q->center_lon - dlon;
        const double right = q->center_lon + dlon;

        if (left >= ctx.min_long && right <= ctx.max_long) {
            *seg1_min_lon = left;
            *seg1_max_lon = right;
            *segment_count = 1;
        } else if (left < ctx.min_long) {
            *seg1_min_lon = ctx.min_long;
            *seg1_max_lon = right;
            *seg2_min_lon = ctx.max_long - (ctx.min_long - left);
            *seg2_max_lon = ctx.max_long;
            queries[1].center_lon = q->center_lon +
                (ctx.max_long - ctx.min_long);
            *segment_count = 2;
        } else {
            *seg1_min_lon = left;
            *seg1_max_lon = ctx.max_long;
            *seg2_min_lon = ctx.min_long;
            *seg2_max_lon = ctx.min_long + (right - ctx.max_long);
            queries[1].center_lon = q->center_lon -
                (ctx.max_long - ctx.min_long);
            *segment_count = 2;
        }

        return;
    }

    const double alpha = q->radius_rad;
    const double phi = q->center_lat_rad;

    if (alpha >= M_PI) {
        *min_lat = ctx.min_lat;
        *max_lat = ctx.max_lat;
        *seg1_min_lon = ctx.min_long;
        *seg1_max_lon = ctx.max_long;
        *segment_count = 1;
        return;
    }

    *min_lat = rad_to_deg(fmax(-M_PI * 0.5, phi - alpha));
    *max_lat = rad_to_deg(fmin( M_PI * 0.5, phi + alpha));

    if (phi + alpha >= M_PI * 0.5 || phi - alpha <= -M_PI * 0.5) {
        *seg1_min_lon = ctx.min_long;
        *seg1_max_lon = ctx.max_long;
        *segment_count = 1;
        return;
    }

    const double c = fabs(cos(phi));
    if (c < 1e-15) {
        *seg1_min_lon = ctx.min_long;
        *seg1_max_lon = ctx.max_long;
        *segment_count = 1;
        return;
    }

    double ratio = sin(alpha) / c;
    ratio = clamp_double(ratio, -1.0, 1.0);
    const double dlon = asin(fabs(ratio));
    const double dlon_deg = rad_to_deg(dlon);

    if (dlon_deg >= 180.0) {
        *seg1_min_lon = ctx.min_long;
        *seg1_max_lon = ctx.max_long;
        *segment_count = 1;
        return;
    }

    const double left = q->center_lon - dlon_deg;
    const double right = q->center_lon + dlon_deg;

    if (left >= ctx.min_long && right <= ctx.max_long) {
        *seg1_min_lon = left;
        *seg1_max_lon = right;
        *segment_count = 1;
    } else if (left < ctx.min_long) {
        *seg1_min_lon = ctx.min_long;
        *seg1_max_lon = right;
        *seg2_min_lon = ctx.max_long - (ctx.min_long - left);
        *seg2_max_lon = ctx.max_long;
        queries[1].center_lon = q->center_lon +
            (ctx.max_long - ctx.min_long);
        *segment_count = 2;
    } else {
        *seg1_min_lon = left;
        *seg1_max_lon = ctx.max_long;
        *seg2_min_lon = ctx.min_long;
        *seg2_max_lon = ctx.min_long + (right - ctx.max_long);
        queries[1].center_lon = q->center_lon -
            (ctx.max_long - ctx.min_long);
        *segment_count = 2;
    }
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

bool spatial_get_radius_ranges(
    double center_lat,
    double center_lon,
    double radius_km,
    SpatialRange *out_ranges,
    int *out_num_ranges,
    int max_ranges,
    SpatialzCtx ctx)
{
    if (!out_ranges || !out_num_ranges || max_ranges <= 0)
        return false;

    *out_num_ranges = 0;

    FastQueryCtx q;
    if (!make_query(center_lat, center_lon, radius_km, ctx, &q))
        return false;

    FastQueryCtx queries[2];
    double min_lat, max_lat;
    double seg1_min_lon, seg1_max_lon;
    double seg2_min_lon = 0.0, seg2_max_lon = 0.0;
    int segment_count = 0;

    get_query_lon_segments(
        &q, ctx,
        &min_lat, &max_lat,
        &seg1_min_lon, &seg1_max_lon,
        &seg2_min_lon, &seg2_max_lon,
        &segment_count,
        queries);

    ZBlock blocks[max_ranges];
    int block_count = 0;

    const int first_count = create_root_blocks(
        seg1_min_lon,
        seg1_max_lon,
        min_lat,
        max_lat,
        0,
        max_ranges,
        ctx,
        queries,
        blocks);
    block_count = first_count;

    if (segment_count == 2 && block_count < max_ranges) {
        block_count += create_root_blocks(
            seg2_min_lon,
            seg2_max_lon,
            min_lat,
            max_lat,
            1,
            max_ranges - block_count,
            ctx,
            queries,
            blocks + block_count);
    }

    if (block_count == 0)
        return false;

    refine_blocks(
        blocks,
        &block_count,
        max_ranges,
        queries,
        ctx);

    int range_count = 0;
    for (int i = 0; i < block_count; ++i) {
        if (blocks[i].cls == BLOCK_OUTSIDE)
            continue;
        out_ranges[range_count++] = encode_block(&blocks[i]);
    }

    range_count = merge_ranges(out_ranges, range_count);

    if (range_count > max_ranges)
        return false;

    *out_num_ranges = range_count;
    return range_count > 0;
}

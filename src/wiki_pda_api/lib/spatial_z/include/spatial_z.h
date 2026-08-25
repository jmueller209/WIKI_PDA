#ifndef SPATIAL_Z_H
#define SPATIAL_Z_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif


/*
 * Maximum depth of the spatial quadtree.
 *
 * A level of 31 means the coordinate axes are divided into 2^31
 * discrete cells. This utilizes a 62 bit of a 64-bit Morton code
 * (31 bits for axis1 + 31 bits for axis2).
 * If running on constrained microcontrollers with 32-bit floats,
 * this can be reduced (e.g., to 22) to prevent precision loss. 
 * Do not use values greater than 31 as this might lead to overflow
 * issues.
 */
#ifndef SPATIALZ_MAX_GRID_LEVEL
#define SPATIALZ_MAX_GRID_LEVEL 22U
#endif

/*
 * The maximum value of the coordinate grid, automatically derived 
 * from SPATIALZ_MAX_GRID_LEVEL (e.g. 2^32 - 1).
 *
 * This mathematically links the scale of the world to the depth 
 * of the quadtree, ensuring they never fall out of sync.
 */
#define GRID_MAX_UINT ((float)((1ULL << SPATIALZ_MAX_GRID_LEVEL) - 1ULL))

/*
 * Maximum number of working blocks allocated on the stack
 * during spatial queries.
 *
 * This acts as a hard memory limit to prevent stack overflows
 * on embedded devices. If a query requests more ranges than
 * this limit, the request is safely clamped to this value.
 */
#ifndef SPATIALZ_MAX_INTERNAL_BLOCKS
#define SPATIALZ_MAX_INTERNAL_BLOCKS 16
#endif

/*
 * The mathematical constant Pi.
 *
 * Used for spherical trigonometry and rad/deg conversions.
 * Can be overridden by the build system if the target platform
 * requires a specific optimized math constant.
 */
#ifndef SPATIALZ_PI
#define SPATIALZ_PI 3.14159265358979323846f
#endif

/*
 * Describes the coordinate system and physical distance model used by
 * the spatial index.
 *
 * The two coordinate axes are angular coordinates measured in degrees.
 *
 * axis1 is the latitude-like coordinate. It always spans exactly 180
 * degrees starting from min_axis1 (e.g., -90 to +90 for a standard globe).
 *
 * axis2 is the longitude-like periodic coordinate. It always spans exactly
 * 360 degrees starting from min_axis2 (e.g., -180 to +180, or 0 to 360).
 *
 * The Spatial-Z implementation does not assign any physical meaning to
 * the axes. The caller decides what they represent.
 */
typedef struct {
    /* 
     * Start of the 180-degree interval for the latitude-like axis.
     */
    float min_axis1;

    /* 
     * Start of the 360-degree interval for the longitude-like axis.
     */
    float min_axis2;

    /*
     * Physical distance represented by one degree of axis1.
     *
     * The unit is chosen entirely by the caller. It may be kilometres,
     * miles, metres, or any other consistent distance unit.
     *
     * For a spherical surface this value also defines the corresponding
     * surface radius through:
     *
     *     surface_radius = unit_length * 180 / PI
     */
    float unit_length;
} SpatialzCtx;


/*
 * Precomputed state for repeatedly testing encoded spatial points against
 * one radius query.
 *
 * The structure is intended for the database search hot path. Expensive
 * values are calculated once when the query starts and then reused for
 * every Morton code that is inspected.
 */
typedef struct {
    SpatialzCtx spatialCtx;

    float center_axis1;
    float center_axis2;
    float radius;

    float center_v[3];
    float radius_chord_sq;
} CompareCtx;


/*
 * Inclusive interval in 64-bit Morton/Z-order space.
 *
 * Every spatial cell represented by the spatial index is translated into
 * one or more intervals of this type.
 */
typedef struct {
    uint64_t start_code;
    uint64_t end_code;
} MortonRange;


/*
 * Creates a generic spatial context.
 *
 * The caller specifies the minimum angular boundaries (the spans are 
 * strictly fixed to 180 degrees for axis1 and 360 degrees for axis2), 
 * and the physical distance represented by one degree of axis1.
 */
SpatialzCtx spatial_create_ctx(
    float min_axis1,
    float min_axis2,
    float unit_length
);

/*
 * Creates a spatial context for geographic coordinates on Earth.
 *
 * Axis 1:
 *   Latitude, starting at -90 degrees (spanning to +90).
 *
 * Axis 2:
 *   Longitude, starting at -180 degrees (spanning to +180).
 *
 * The unit length is approximately 111.3195 km per degree
 * of latitude at the Earth's surface.
 */
SpatialzCtx spatial_create_earth_ctx(void);

/*
 * Creates a spatial context for a generic celestial coordinate
 * system using two angular axes.
 *
 * Axis 1:
 *   Declination-like angle, starting at -90 degrees (spanning to +90).
 *
 * Axis 2:
 *   Right-ascension-like angle, starting at 0 degrees (spanning to 360).
 *
 * The unit length is set to 1.0 distance unit per degree.
 * The caller is responsible for interpreting this distance unit.
 */
SpatialzCtx spatial_create_celestial_ctx(void);


/*
 * Encodes two angular coordinates into a 64-bit Morton/Z-order code.
 *
 * axis1 is mapped to the first spatial grid dimension.
 *
 * axis2 is mapped to the second spatial grid dimension.
 *
 * The encoding is independent of the physical distance unit and of the
 * geometric interpretation of the two axes. Coordinates exceeding the 
 * standard 180/360 degree bounds are automatically wrapped using pure 
 * spherical geometry before encoding.
 */
uint64_t spatial_encode(
    float axis1,
    float axis2,
    SpatialzCtx ctx
);


/*
 * Decodes a 64-bit Morton/Z-order code back into the two angular
 * coordinates represented by the SpatialzCtx.
 *
 * Returns false when an output pointer is NULL.
 */
bool spatial_decode(
    uint64_t code,
    float *out_axis1,
    float *out_axis2,
    SpatialzCtx ctx
);


/*
 * Generates Morton/Z-order ranges covering a radius query.
 *
 * The returned ranges are guaranteed to represent a conservative spatial
 * cover of the requested query. The function never intentionally removes
 * an intersecting spatial block merely because it is not fully contained
 * by the radius.
 *
 * max_ranges specifies the maximum number of returned ranges.
 *
 * radius is expressed in the physical distance unit defined by
 * SpatialzCtx.unit_length.
 */
bool spatial_get_radius_ranges(
    float center_axis1,
    float center_axis2,
    float radius,
    MortonRange *out_ranges,
    int *out_num_ranges,
    int max_ranges,
    const SpatialzCtx *ctx
);


/*
 * Creates a prepared comparison context for one radius query.
 *
 * The geometric comparison uses 3D vector spherical geometry.
 * No Earth-specific or unit-specific information is required here.
 */
CompareCtx spatial_create_compare_ctx(
    float center_axis1,
    float center_axis2,
    float radius,
    SpatialzCtx spatialCtx
);


/*
 * Tests whether a Morton code lies within a given radius.
 */
float spatial_code_is_in_radius(
    uint64_t code,
    const CompareCtx *ctx
);

#ifdef __cplusplus
}
#endif

#endif

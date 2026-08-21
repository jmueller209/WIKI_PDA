#ifndef SPATIAL_Z_H
#define SPATIAL_Z_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    double min_lat;
    double max_lat;
    double min_long;
    double max_long;
    double unit_length; // distance between 1 degree of latitude: Used for calculating distances
} SpatialzCtx;

typedef struct {
    SpatialzCtx spatialCtx;
    union {
        // Only used for local comparisons
        struct {
            double lat;
            double lon;
            double km_per_deg_lat;
            double km_per_deg_lon;
            double radius_squared;
        } local;

        // Only used for spherical comparisons
        struct {
            double center_lat_rad;
            double center_lon_rad;
            double cos_center_lat;
            double max_haversine_a;
        } spherical;
    };
} CompareCtx;

typedef struct {
    uint64_t start_code;
    uint64_t end_code;
} SpatialRange;

// Creates and initializes a SpatialzCtx with custom boundaries and unit length.
SpatialzCtx spatial_create_ctx(double min_lat, double max_lat, double min_lon, double max_lon, double unit_length);

// Creates a standard SpatialzCtx for Earth (-90 to 90 lat, -180 to 180 lon, 111.3195 km per degree).
SpatialzCtx spatial_create_earth_ctx(void);

// Creates a standard SpatialzCtx for the Celestial Sphere (-90 to 90 Dec, 0 to 360 RA, 1.0 units per degree).
SpatialzCtx spatial_create_celestial_ctx(void);

// Encodes lat/lon into a 64-bit Morton code using the context bounds
uint64_t spatial_encode(double lat, double lon, SpatialzCtx ctx);

// Decodes a 64-bit Morton code back into lat/lon 
bool spatial_decode(uint64_t code, double* out_lat, double* out_long, SpatialzCtx ctx);

// Calculates 1D Morton code ranges for a radius search
bool spatial_get_radius_ranges(double center_lat, double center_lon, double radius_km, 
                               SpatialRange* out_ranges, int* out_num_ranges, int max_ranges, 
                               SpatialzCtx ctx);

// Initializes the context for point-in-radius checks, pre-calculating math based on the chosen mode.
CompareCtx spatial_create_compare_ctx(double center_lat, double center_lon, double radius_km, bool is_spherical, SpatialzCtx spatialCtx);

// Fast Euclidean distance check for small radii (uses the local struct fields).
bool spatial_code_is_in_local_radius(uint64_t code, CompareCtx ctx);

// Accurate Haversine distance check for large radii (uses the spherical struct fields).
bool spatial_code_is_in_spherical_radius(uint64_t code, CompareCtx ctx);


#ifdef __cplusplus
}
#endif

#endif // SPATIAL_Z_H

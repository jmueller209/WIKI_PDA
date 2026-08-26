use std::ffi::CString;
use std::os::raw::{c_char, c_float};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SpatialzCtx {
    pub min_axis1: c_float,
    pub min_axis2: c_float,
    pub unit_length: c_float,
}

unsafe extern "C" {
    // Creates a standard SpatialzCtx for Earth
    fn spatial_create_earth_ctx() -> SpatialzCtx;

    // Creates a standard SpatialzCtx for the Celestial Sphere
    fn spatial_create_celestial_ctx() -> SpatialzCtx;

    // Encodes lat/lon into a 64-bit Morton code using the context bounds
    fn spatial_encode(axis1: c_float, axis2: c_float, ctx: SpatialzCtx) -> u64;

    // Encodes data strings into a 64-bit representation
    fn temporal_encode(iso_str: *const c_char) -> i64;
}

pub fn safe_temporal_encode(iso_str: &str) -> i64 {
    if let Ok(c_string) = CString::new(iso_str) {
        unsafe { temporal_encode(c_string.as_ptr()) }
    } else {
        -1
    }
}

pub fn safe_spatial_encode(lat_or_dec: f32, lon_or_ra: f32, ctx: SpatialzCtx) -> u64 {
    unsafe { spatial_encode(lat_or_dec as c_float, lon_or_ra as c_float, ctx) }
}

pub fn safe_spatial_create_earth_ctx() -> SpatialzCtx {
    unsafe { spatial_create_earth_ctx() }
}

pub fn safe_spatial_create_celestial_ctx() -> SpatialzCtx {
    unsafe { spatial_create_celestial_ctx() }
}


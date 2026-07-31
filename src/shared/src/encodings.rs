use std::ffi::CString;
use std::os::raw::c_char;

unsafe extern "C" {
    fn encode_time(iso_str: *const c_char) -> i64;
    fn encode_globe_coordinates(lat: f64, lon: f64) -> i64;
    fn encode_astronomical_position(dec: f64, ra: f64) -> i64;
}

pub fn safe_encode_time(iso_str: &str) -> i64 {
    if let Ok(c_string) = CString::new(iso_str) {
        unsafe { encode_time(c_string.as_ptr()) }
    } else {
        -1
    }
}

pub fn safe_encode_globe_coordinates(lat: f64, lon: f64) -> i64 {
    unsafe { encode_globe_coordinates(lat, lon) }
}

pub fn safe_encode_astronomical_position(dec: f64, ra: f64) -> i64 {
    unsafe { encode_astronomical_position(dec, ra) }
}

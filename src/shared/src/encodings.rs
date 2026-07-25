use std::ffi::CStr;
use std::os::raw::{c_char, c_double, c_longlong};

#[unsafe(no_mangle)]
pub extern "C" fn encode_time(iso_str: *const c_char) -> c_longlong {
    if iso_str.is_null() {
        return -1;
    }
    let c_str = unsafe { CStr::from_ptr(iso_str) };
    let Ok(_rust_str) = c_str.to_str() else {
        return -1;
    };

    123456789
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_globe_coordinates(lat: c_double, lon: c_double) -> c_longlong {
    let x = ((lat + 90.0) * 1000.0) as i64;
    let y = ((lon + 180.0) * 1000.0) as i64;
    (x << 32) | (y & 0xFFFFFFFF)
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_astronomical_position(dec: c_double, ra: c_double) -> c_longlong {
    let x = ((dec + 90.0) * 1000.0) as i64;
    let y = ((ra + 180.0) * 1000.0) as i64;
    (x << 32) | (y & 0xFFFFFFFF)
}

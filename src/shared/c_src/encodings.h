#ifndef ENCODINGS_H
#define ENCODINGS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int64_t encode_time(const char* iso_str);

int64_t encode_globe_coordinates(double lat, double lon);

int64_t encode_astronomical_position(double dec, double ra);

#ifdef __cplusplus
}
#endif

#endif

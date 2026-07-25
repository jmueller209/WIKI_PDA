#ifndef SHARED_LIB_H
#define SHARED_LIB_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

long long encode_time(const char *iso_str);

long long encode_globe_coordinates(double lat, double lon);

long long encode_astronomical_position(double dec, double ra);

#endif  /* SHARED_LIB_H */

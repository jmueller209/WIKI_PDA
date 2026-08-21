#ifndef TEMPUS_H
#define TEMPUS_H

#include <stdio.h>
#include <inttypes.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

int64_t temporal_encode(const char* iso_str);

bool temporal_decode(int64_t code, const char* out_iso_str);

#ifdef __cplusplus
}
#endif

#endif //TEMPUS_H

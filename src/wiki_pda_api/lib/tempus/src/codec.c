#include "../include/tempus.h"
#include <stdio.h>
#include <inttypes.h>
#include <stdbool.h>

int64_t temporal_encode(const char* iso_str) {
    if (iso_str == NULL || iso_str[0] == '\0') {
        return -1;
    }

    char sign;
    int64_t year = 0, month = 0, day = 0;

    if (sscanf(iso_str, "%c%" SCNd64 "-%" SCNd64 "-%" SCNd64, &sign, &year, &month, &day) < 4) {
        return -1;
    }

    if (sign == '-') {
        year = -year;
    }

    int64_t sortable_date = (year * 10000) + (month * 100) + day;

    return sortable_date;
}

bool temporal_decode(int64_t code, const char* out_iso_str){
    return false;
}

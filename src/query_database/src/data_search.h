#ifndef ARTICLE_SEARCH_H
#define ARTICLE_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "database_constants.h"
#include "database_io.h"

bool get_data(uint64_t data_offset, uint32_t data_length, uint8_t** out_data, bool is_metadata);
#endif

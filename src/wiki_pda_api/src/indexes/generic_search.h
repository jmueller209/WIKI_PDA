#ifndef GENERIC_SEARCH_H
#define GENERIC_SEARCH_H

#include <stdint.h>
#include <stdbool.h>
#include "../../include/database_platform.h"



bool load_top_level_index_generic(void** out_index,
                              uint32_t row_count,
                              size_t row_size,
                              uint64_t offset,
                              DatabasePlatform platform,
                              const char* index_name);



#endif // GENERIC_SEARCH_H

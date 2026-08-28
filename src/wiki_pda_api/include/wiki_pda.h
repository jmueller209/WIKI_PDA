#ifndef KNOWLEDGE_API_H
#define KNOWLEDGE_API_H

#ifdef __cplusplus
extern "C" {
#endif

#include "wiki_pda_types.h"
#include "wiki_pda_options.h"
#include "wiki_pda_platforms.h"

DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform);

bool db_end(DatabaseContext* ctx);

SearchCursor* search_begin(DatabaseContext* ctx, const SearchQuery* query);

bool search_next(SearchCursor* cursor, SearchResult* out_result);

bool search_end(SearchCursor* cursor);

DataStream* data_stream_begin(DatabaseContext* ctx, uint64_t data_offset, uint32_t data_length);

bool data_stream_read(DataStream* stream, char* out_buffer, uint32_t buffer_capacity, uint32_t* out_bytes_read);

bool data_stream_end(DataStream* stream);

#ifdef __cplusplus
}
#endif

#endif

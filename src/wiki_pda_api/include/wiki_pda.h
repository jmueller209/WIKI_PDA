/**
 * @file knowledge_api.h
 * @brief Core API for the Wiki PDA Database Engine.
 *
 * This header exposes the main lifecycle, querying, and data streaming functions 
 * required to interact with the offline Wikipedia database.
 */

#ifndef KNOWLEDGE_API_H
#define KNOWLEDGE_API_H

#ifdef __cplusplus
extern "C" {
#endif

#include "wiki_pda_types.h"
#include "wiki_pda_options.h"
#include "wiki_pda_platforms.h"

/**
 * @brief Initializes the database context and loads requested indexes into memory.
 *
 * This function validates the database header, loads the ZSTD dictionary, 
 * and allocates memory for the requested search indexes (e.g., text, spatial, temporal).
 * 
 * @param indexes_to_load A bitmask (DatabaseIndexMask) specifying which indexes to load.
 * @param platform The platform abstraction struct providing the read function and user data.
 * @return A pointer to the initialized DatabaseContext on success, or NULL on failure.
 */
DatabaseContext* db_init(DatabaseIndexMask indexes_to_load, DatabasePlatform platform);

/**
 * @brief Cleans up and frees the database context.
 *
 * Safely destroys all loaded top-level indexes, the ZSTD dictionary context, 
 * and frees the memory associated with the DatabaseContext.
 *
 * @param ctx Pointer to the active database context.
 * @return true if successfully closed, false if the context was NULL.
 */
bool db_end(DatabaseContext* ctx);

/**
 * @brief Initiates a new search based on the provided query.
 *
 * Evaluates the query parameters (type, target, filters) and prepares an internal
 * search state (e.g., calculating spatial Morton ranges or setting up string matching).
 *
 * @param ctx Pointer to the active database context.
 * @param query Pointer to the SearchQuery configuration.
 * @return A pointer to a newly allocated SearchCursor, or NULL if the query is invalid
 *         or the required index was not loaded.
 */
SearchCursor* search_begin(DatabaseContext* ctx, const SearchQuery* query);

/**
 * @brief Fetches the next matching result from the active search cursor.
 *
 * Acts as an iterator. It scans the database incrementally (or yields from a sorted
 * Top-K buffer) without loading all results into memory at once.
 *
 * @param cursor Pointer to the active SearchCursor.
 * @param out_result Pointer to a SearchResult struct where the matched data will be written.
 * @return true if a result was found and written to out_result. false if there are no 
 *         more results (end of search) or the cursor is invalid.
 */
bool search_next(SearchCursor* cursor, SearchResult* out_result);

/**
 * @brief Ends a search and frees the associated cursor.
 *
 * Must be called when a search is finished or aborted to prevent memory leaks.
 *
 * @param cursor Pointer to the SearchCursor to destroy.
 * @return true if successfully freed, false if the cursor was NULL.
 */
bool search_end(SearchCursor* cursor);

/**
 * @brief Opens a data stream to read (and potentially decompress) an article or metadata payload.
 *
 * Automatically detects whether the requested data region is compressed (content) 
 * or uncompressed (metadata) based on the database header and sets up the ZSTD 
 * decompression context if necessary.
 *
 * @param ctx Pointer to the active database context.
 * @param data_offset Absolute physical offset of the payload in the database file.
 * @param data_length Total length of the compressed (or raw) payload on disk.
 * @return A pointer to an initialized DataStream, or NULL if parameters are out of bounds.
 */
DataStream* data_stream_begin(DatabaseContext* ctx, uint64_t data_offset, uint32_t data_length);

/**
 * @brief Reads a chunk of uncompressed data from the stream into the provided buffer.
 *
 * For compressed payloads, this function automatically reads from the disk in chunks
 * and passes them through the ZSTD decompressor. It should be called in a loop until
 * it returns false.
 *
 * @param stream Pointer to the active DataStream.
 * @param out_buffer Pointer to the user-allocated memory buffer to hold the output.
 * @param buffer_capacity Maximum number of bytes that can be written to out_buffer.
 * @param out_bytes_read Pointer to a variable where the actual number of bytes written will be stored.
 * @return true if data was successfully read and output produced. false if the stream
 *         has reached EOF (no more data) or a read/decompression error occurred.
 */
bool data_stream_read(DataStream* stream, char* out_buffer, uint32_t buffer_capacity, uint32_t* out_bytes_read);

/**
 * @brief Closes a data stream and frees internal buffers/decompression contexts.
 *
 * @param stream Pointer to the DataStream to destroy.
 * @return true if successfully closed, false if the stream was NULL.
 */
bool data_stream_end(DataStream* stream);

#ifdef __cplusplus
}
#endif

#endif // KNOWLEDGE_API_H

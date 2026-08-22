 // Used to make sure QIDs are not considered multiple times when searching (the omni search index).
#define MAX_DEDUPLICATION_CACHE 128

// Maximum number of results saved when performing sorted spatial queries (relevant for sorted astronomical and sorted globe coordinate search).
#define MAX_SORTED_RESULTS 50

// Maximum number of morton ranges that need to be searched. A lower number reduces the number of random reads the database needs to perform.
// Increasing the number will increase the number of sequential reads needed (relevant for astronomical and globe coordinate search).
#define MAX_MORTON_RANGES 12

// Search radius at which the API starts considering the curvature of the earth for distance calculation (relevant only for globe coordinate search).
#define LOCAL_SEARCH_LIMIT_KM 500

#if !defined(ARDUINO) && !defined(ESP32)
#include "../../include/platform_providers.h"


static bool standard_file_read(uint64_t offset, uint8_t* buf, uint32_t len, void* user_data) {
    FILE* f = (FILE*)user_data;
    if (f == NULL) {
        return false;
    }
    if (fseeko(f, offset, SEEK_SET) != 0) {
        return false;
    }
    size_t bytes_read = fread(buf, 1, len, f);
    return bytes_read == len;
}

DatabasePlatform platform_desktop(FILE* f) {
    DatabasePlatform platform;
    platform.read_fn = standard_file_read;
    platform.user_data = f;
    return platform;
}

#endif // desktop

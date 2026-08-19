#ifndef PLATFORM_PROVIDERS_H
#define PLATFORM_PROVIDERS_H

#include "database_platform.h"

#ifdef __cplusplus
extern "C" {
#endif

// Always available for PC / Desktop
DatabasePlatform platform_desktop(FILE* f);

#if defined(ARDUINO) || defined(TEENSYDUINO) || defined(ESP32)
    #ifdef __cplusplus
        class SdCardInterface;
        typedef SdCardInterface SdCard;
    #else
        typedef struct SdCardInterface SdCard;
    #endif

    DatabasePlatform platform_teensy(SdCard* c);
    DatabasePlatform platform_esp32(SdCard* c);
#endif

#ifdef __cplusplus
}
#endif

#endif // PLATFORM_PROVIDERS_H

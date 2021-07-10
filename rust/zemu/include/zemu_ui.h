#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(TARGET_NANOS)
uint8_t *viewdata_key(void);
#endif

#if defined(TARGET_NANOS)
uint8_t *viewdata_message_line1(void);
#endif

#if defined(TARGET_NANOS)
uint8_t *viewdata_message_line2(void);
#endif

#if defined(TARGET_NANOX)
uint8_t *viewdata_key(void);
#endif

#if defined(TARGET_NANOX)
uint8_t *viewdata_message(void);
#endif

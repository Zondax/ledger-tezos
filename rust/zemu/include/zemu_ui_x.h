#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NanoXBackend {
  ArrayString<KEY_SIZE> key;
  ArrayString<MESSAGE_SIZE> message;
  uintptr_t viewable_size;
  bool expert;
} NanoXBackend;

extern struct NanoXBackend BACKEND_LAZY;

uint8_t *viewdata_key(void);

uint8_t *viewdata_message(void);

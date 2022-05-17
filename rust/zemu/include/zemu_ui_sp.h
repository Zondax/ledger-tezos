#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define KEY_SIZE (63 + 1)

#define MESSAGE_SIZE (4095 + 1)

typedef struct NanoSPBackend {
  uint8_t key[KEY_SIZE];
  uint8_t message[MESSAGE_SIZE];
  uintptr_t viewable_size;
  bool expert;
  bool flow_inside_loop;
} NanoSPBackend;

extern struct NanoSPBackend BACKEND_LAZY;

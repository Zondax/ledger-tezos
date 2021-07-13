#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define KEY_SIZE (17 + 1)

#define MESSAGE_LINE_SIZE (17 + 1)

typedef struct NanoSBackend {
  uint8_t key[KEY_SIZE];
  uint8_t value[MESSAGE_LINE_SIZE];
  uint8_t value2[MESSAGE_LINE_SIZE];
  uintptr_t viewable_size;
  bool expert;
} NanoSBackend;

extern struct NanoSBackend BACKEND_LAZY;

void rs_h_expert_toggle(void);

bool rs_h_paging_can_decrease(void);

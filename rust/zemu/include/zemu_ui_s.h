#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct NanoSBackend {
  ArrayString<KEY_SIZE> key;
  ArrayString<MESSAGE_LINE_SIZE> message_line1;
  ArrayString<MESSAGE_LINE_SIZE> message_line2;
  uintptr_t viewable_size;
  bool expert;
} NanoSBackend;

extern struct NanoSBackend BACKEND_LAZY;

uint8_t *viewdata_key(void);

uint8_t *viewdata_message_line1(void);

uint8_t *viewdata_message_line2(void);

void rs_h_expert_toggle(void);

bool rs_h_paging_can_decrease(void);

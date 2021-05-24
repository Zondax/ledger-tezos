#pragma once

#include <stdint.h>

void check_canary();

void zemu_log(const char *buf);

void
rs_handle_apdu(volatile uint32_t *flags, volatile uint32_t *tx, uint32_t rx, const uint8_t *buffer, uint16_t bufferLen);

/////////////

// FIXME: Refactor these two

void view_init();

void app_init();

#ifndef WRAPPERX_H_
#define WRAPPERX_H_

#include "defs.h"

#include "bolos_version.h"

// Taken from Makefile
#define IO_SEPROXYHAL_BUFFER_SIZE_B 300
#define HAVEGLO096

#define HAVE_BAGL
#define BAGL_WIDTH 128
#define BAGL_HEIGHT 64
#define HAVE_BAGL_ELLIPSIS
#define HAVE_BAGL_FONT_OPEN_SANS_REGULAR_11PX
#define HAVE_BAGL_FONT_OPEN_SANS_EXTRABOLD_11PX
#define HAVE_BAGL_FONT_OPEN_SANS_LIGHT_16PX

#define HAVE_UX_FLOW

#define HAVE_BLE
#define HAVE_BLE_APDU
#define BLE_COMMAND_TIMEOUT_MS 2000

#include "wrapper.h"
#include "cx.h"

#endif // WRAPPERX_H_

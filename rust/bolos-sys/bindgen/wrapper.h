#ifndef WRAPPER_H_
#define WRAPPER_H_

// Taken from Makefile

#define PRINTF(...)

#define OS_IO_SEPROXYHAL

#define HAVE_BAGL
#define HAVE_SPRINTF

#define HAVE_IO_USB
#define HAVE_L4_USBLIB
#define IO_USB_MAX_ENDPOINTS 7
#define IO_HID_EP_LENGTH 64
#define HAVE_USB_APDU

#define USB_SEGMENT_SIZE 64
#define HAVE_BOLOS_APP_STACK_CANARY

#define HAVE_WEBUSB
#define WEBUSB_URL_SIZE_B 0
#define WEBUSB_URL ""

#include "bagl.h"
#include "bolos_target.h"
#include "bolos_version.h"

#include "os_hal.h"
#include "os_io_seproxyhal.h"
#include "os_io_usb.h"
#include "os.h"

#endif // WRAPPER_H_

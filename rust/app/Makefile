#*******************************************************************************
#   Ledger App
#   (c) 2018-2020 Zondax GmbH
#   (c) 2017 Ledger
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
#*******************************************************************************

ifeq ($(BOLOS_SDK),)
$(error BOLOS_SDK is not set)
endif

MY_DIR := $(dir $(lastword $(MAKEFILE_LIST)))

all: bin/app.elf
	@echo "#!/usr/bin/env bash" > $(OUTPUT_INSTALLER)
	@echo "APPNAME=\"${APPNAME}\"" >> $(OUTPUT_INSTALLER)
	@echo "APPVERSION=\"${APPVERSION}\"" >> $(OUTPUT_INSTALLER)
	@echo "APPPATH=\""${APPPATH}"\"" >> $(OUTPUT_INSTALLER)
	@echo "LOAD_PARAMS=($$(echo "${APP_LOAD_PARAMS}" | sed -e "s|"${APPNAME}"|\""${APPNAME}"\"|"))" >> $(OUTPUT_INSTALLER)
	@echo "DELETE_PARAMS=($$(echo "${COMMON_DELETE_PARAMS}" | sed -e "s|"${APPNAME}"|\""${APPNAME}"\"|"))" >> $(OUTPUT_INSTALLER)
	@echo "APPHEX=\"" >> $(OUTPUT_INSTALLER)
	@cat $(CURDIR)/bin/app.hex >> $(OUTPUT_INSTALLER)
	@echo "\"" >> $(OUTPUT_INSTALLER)
	@cat $(CURDIR)/refactor/template.sh >> $(OUTPUT_INSTALLER)
	@chmod +x $(OUTPUT_INSTALLER)
	@cp $(CURDIR)/bin/* $(CURDIR)/output
	@cp $(CURDIR)/output/app.elf ${OUTPUT_ELF}
	@rm $(CURDIR)/output/app.elf

include $(BOLOS_SDK)/Makefile.defines

DEFINES += APP_SECRET_MODE_ENABLED

$(info ************ TARGET_NAME  = [$(TARGET_NAME)])

ifeq ($(APP_TESTING),1)
DEFINES += APP_TESTING
DEFINES += ZEMU_LOGGING
$(info ************ LOGGING ENABLED ************)
endif

ifndef COIN
COIN=XTZ
endif

include $(CURDIR)/Makefile.version

$(info COIN  = [$(COIN)])
ifeq ($(COIN),XTZ)
# Main app configuration
DEFINES += APP_STANDARD
ifdef BAKING
APPNAME = "Tezos Baking"
else
APPNAME = "Tezos"
endif
APPPATH = "44'/1729'"

else ifeq ($(COIN),XTZ_baker)
else
define error_message

COIN value not supported: [$(COIN)]

endef
$(error "$(error_message)")
endif

APP_LOAD_PARAMS = $(COMMON_LOAD_PARAMS) --path ${APPPATH} --path "44'/1'" \
					--curve ed25519 --curve secp256k1 --curve secp256r1

ifeq ($(TARGET_NAME),TARGET_NANOS)
APP_STACK_SIZE:=3072
ICONNAME:=$(CURDIR)/nanos_icon.gif
OUTPUT_ELF ?= $(CURDIR)/output/app_s.elf
OUTPUT_INSTALLER := $(CURDIR)/pkg/installer_s.sh
RUST_TARGET := thumbv6m-none-eabi
APP_LOAD_PARAMS += --appFlags 0x200
endif

ifeq ($(TARGET_NAME),TARGET_NANOX)
ICONNAME:=$(CURDIR)/nanox_icon.gif
OUTPUT_ELF ?= $(CURDIR)/output/app_x.elf
OUTPUT_INSTALLER:= $(CURDIR)/pkg/installer_x.sh
RUST_TARGET := thumbv6m-none-eabi
APP_LOAD_PARAMS += --appFlags 0x200
endif

ifeq ($(TARGET_NAME),TARGET_NANOS2)
ICONNAME:=$(CURDIR)/nanox_icon.gif
OUTPUT_ELF ?= $(CURDIR)/output/app_sp.elf
OUTPUT_INSTALLER:= $(CURDIR)/pkg/installer_sp.sh
# RUST_TARGET := thumbv8m.main-none-eabi
RUST_TARGET := thumbv6m-none-eabi
APP_LOAD_PARAMS += --appFlags 0x000
endif

ifdef BAKING
ifeq ($(TARGET_NAME),TARGET_NANOX)
$(error "Baking is not supported with a Nano X device")
endif
RUST_FEATURES=--features="baking"
OUTPUT_ELF:=$(subst .elf,_baking.elf,$(OUTPUT_ELF))
OUTPUT_INSTALLER:=$(subst installer,installer_baking,$(OUTPUT_INSTALLER))
BAKING=true
DEFINES += BAKING
else
RUST_FEATURES=--features="wallet"
BAKING=false
endif

ifeq ($(APP_TESTING),1)
RUST_FEATURES:=$(RUST_FEATURES),"dev"
endif

$(info BAKING = [$(BAKING)])
$(info TARGET_NAME  = [$(TARGET_NAME)])
$(info ICONNAME  = [$(ICONNAME)])
$(info OUTPUT_ELF = [$(OUTPUT_ELF)])
$(info OUTPUT_INSTALLR = [$(OUTPUT_INSTALLER)])

ifndef ICONNAME
$(error ICONNAME is not set)
endif

############
# Platform

DEFINES       += UNUSED\(x\)=\(void\)x
DEFINES       += PRINTF\(...\)=

APPVERSION=$(APPVERSION_M).$(APPVERSION_N).$(APPVERSION_P)
DEFINES       += APPVERSION=\"$(APPVERSION)\"

DEFINES       += OS_IO_SEPROXYHAL
DEFINES       += HAVE_BAGL HAVE_SPRINTF
DEFINES       += HAVE_IO_USB HAVE_L4_USBLIB IO_USB_MAX_ENDPOINTS=7 IO_HID_EP_LENGTH=64 HAVE_USB_APDU

DEFINES       += LEDGER_MAJOR_VERSION=$(APPVERSION_M) LEDGER_MINOR_VERSION=$(APPVERSION_N) LEDGER_PATCH_VERSION=$(APPVERSION_P)

DEFINES       += USB_SEGMENT_SIZE=64
DEFINES       += HAVE_BOLOS_APP_STACK_CANARY

DEFINES       += HAVE_WEBUSB WEBUSB_URL_SIZE_B=0 WEBUSB_URL=""

DEFINES		  += HAVE_ECC
DEFINES       += HAVE_ECC_TWISTED_EDWARDS HAVE_ED25519_CURVE
DEFINES		  += HAVE_ECC_WEIERSTRASS HAVE_SECP256K1_CURVE HAVE_SECP256R1_CURVE

DEFINES		  += HAVE_ECDSA HAVE_EDDSA

DEFINES		  += HAVE_HASH HAVE_BLAKE2 HAVE_SHA256 HAVE_SHA512

ifeq ($(TARGET_NAME),TARGET_NANOS)
DEFINES		  += HAVE_BAGL BAGL_WIDTH=128 BAGL_HEIGHT=32
DEFINES		  += BAGL_WIDTH_MARGIN=0
DEFINES       += IO_SEPROXYHAL_BUFFER_SIZE_B=128
else
# Assume Nano X/S+
DEFINES       += IO_SEPROXYHAL_BUFFER_SIZE_B=300

DEFINES       += HAVE_GLO096
DEFINES       += HAVE_BAGL BAGL_WIDTH=128 BAGL_HEIGHT=64
DEFINES       += HAVE_BAGL_ELLIPSIS # long label truncation feature
DEFINES       += HAVE_BAGL_FONT_OPEN_SANS_REGULAR_11PX
DEFINES       += HAVE_BAGL_FONT_OPEN_SANS_EXTRABOLD_11PX
DEFINES       += HAVE_BAGL_FONT_OPEN_SANS_LIGHT_16PX

DEFINES       += HAVE_UX_FLOW
endif

ifeq ($(TARGET_NAME), TARGET_NANOX)
# X specific
DEFINES       += HAVE_BLE
DEFINES       += HAVE_BLE_APDU BLE_COMMAND_TIMEOUT_MS=2000

SDK_SOURCE_PATH  += lib_blewbxx lib_blewbxx_impl
endif

#Feature temporarily disabled
DEFINES   += LEDGER_SPECIFIC

# Compiler, assembler, and linker

ifneq ($(BOLOS_ENV),)
$(info BOLOS_ENV is $(BOLOS_ENV))
CLANGPATH := /usr/bin/
GCCPATH := /usr/bin/
else
$(info BOLOS_ENV is not set: falling back to CLANGPATH and GCCPATH)
endif

ifeq ($(CLANGPATH),)
$(info CLANGPATH is not set: clang will be used from PATH)
endif

ifeq ($(GCCPATH),)
$(info GCCPATH is not set: arm-none-eabi-* will be used from PATH)
endif

#########################

CC := $(CLANGPATH)clang
CFLAGS += -O3 -Os -Wno-unknown-pragmas -Wno-unused-parameter

AS := $(GCCPATH)arm-none-eabi-gcc
AFLAGS +=

LD       := $(GCCPATH)arm-none-eabi-gcc
LDFLAGS  += -O3 -Os
LDLIBS   += -lrslib
LDLIBS   += -lm -lgcc -lc
LDLIBS   += -L../target/$(RUST_TARGET)/release

##########################
GLYPH_SRC_DIR = glyphs
INCLUDES_PATH += $(MY_DIR)/glyphs
include $(BOLOS_SDK)/Makefile.glyphs

APP_SOURCE_PATH += $(MY_DIR)/c
APP_SOURCE_PATH += $(MY_DIR)/refactor

SDK_SOURCE_PATH += lib_stusb lib_stusb_impl
SDK_SOURCE_PATH += lib_ux

.PHONY: rust
rust:
	CARGO_HOME="$(CURDIR)/.cargo" TARGET_NAME=$(TARGET_NAME) cargo build --target $(RUST_TARGET) --release $(RUST_FEATURES)

.PHONY: rust_clean
rust_clean:
	CARGO_HOME="$(CURDIR)/.cargo" cargo clean

clean: rust_clean

# Before linking, we need to be sure rust lib is there
bin/app.elf: rust

# load, delete and listvariants are provided to comply with Ledger requirements
.PHONY: load
load:
	python -m ledgerblue.loadApp $(APP_LOAD_PARAMS)

.PHONY: delete
delete:
	python -m ledgerblue.deleteApp $(COMMON_DELETE_PARAMS)

# Import generic rules from the SDK
include $(BOLOS_SDK)/Makefile.rules

#add dependency on custom makefile filename
dep/%.d: %.c Makefile

.PHONY: listvariants
listvariants:
	@echo VARIANTS COIN XTZ

.PHONY: version
version:
	@echo "v$(APPVERSION)" > app.version

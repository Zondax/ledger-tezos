.PHONY: test

ifdef BAKING
$(info *************** BAKING ENABLED *****************)
RUST_FEATURES=--features="baking"
else
$(info ************* BAKING DISABLED ******************)
RUST_FEATURES=--features="wallet"
endif

$(info BOLOS_SDK = [$(BOLOS_SDK)])

test:
	cargo test $(RUST_FEATURES),"dev"


ifeq ($(TARGET_NAME),TARGET_NANOX)
generate:
	- rm bolos/src/bindings/bindingsX.rs
	bindgen --use-core \
			--ctypes-prefix cty \
			-o bolos/src/bindings/bindingsX.rs \
			bolos/bindgen/wrapperX.h -- \
			-I$(BOLOS_SDK)/include \
			-I$(BOLOS_SDK)/lib_ux/include \
			-Ibolos/bindgen/include \
			-target thumbv6-none-eabi \
			-mcpu=cortex-m0 -mthumb
else ifeq ($(TARGET_NAME),TARGET_NANOS)
generate:
	- rm bolos/src/bindings/bindingsS.rs
	bindgen --use-core \
		--ctypes-prefix cty \
		-o bolos/src/bindings/bindingsS.rs \
		bolos/bindgen/wrapperS.h -- \
		-I$(BOLOS_SDK)/include \
		-I$(BOLOS_SDK)/lib_ux/include \
		-Ibolos/bindgen/include \
		-target thumbv6-none-eabi \
		-mcpu=cortex-m0 -mthumb
else
generate:
	$(error invalid TARGET_NAME (not TARGET_NANOS or TARGET_NANOX))
	$(error TARGET_NAME = [$(TARGET_NAME)])
endif

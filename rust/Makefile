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


.PHONY: prep_generate
prep_generate:
	- mkdir /tmp/bolos/arm-none-eabi
	@echo 'Please make sure `/tmp/bolos/arm-none-eabi` is populated correctly'
	@echo "To populate it: run the current builder-bolos image with the following command:"
	@echo -e 'docker run --rm \
	-v /tmp/bolos:/shared \
	DOCKER_IMAGE \
	cp -r /opt/bolos/gcc-arm-none-eabi-10-2020-q4-major/arm-none-eabi/include /shared/arm-none-eabi/'
	@echo 'You should replace `DOCKER_IMAGE` with the curernt image and also the gcc archive path (after `/opt/bolos` and before `/arm-none-eabi`)'

ifeq ($(TARGET_NAME),TARGET_NANOX)
generate:
	- rm bolos/src/bindings/bindingsX.rs
	bindgen --use-core \
			--with-derive-default \
			--ctypes-prefix cty \
			-o bolos/src/bindings/bindingsX.rs \
			bolos/bindgen/wrapperX.h -- \
			-I$(BOLOS_SDK)/include \
			-I$(BOLOS_SDK)/lib_ux/include \
			-I$(BOLOS_SDK)/lib_cxng/include \
			-I/tmp/bolos/arm-none-eabi/include \
			-Ibolos/bindgen/include \
			-target thumbv6-none-eabi \
			-mcpu=cortex-m0 -mthumb
else ifeq ($(TARGET_NAME),TARGET_NANOS)
generate:
	- rm bolos/src/bindings/bindingsS.rs
	bindgen --use-core \
		--with-derive-default \
		--ctypes-prefix cty \
		-o bolos/src/bindings/bindingsS.rs \
		bolos/bindgen/wrapperS.h -- \
		-I$(BOLOS_SDK)/include \
		-I$(BOLOS_SDK)/lib_ux/include \
		-I$(BOLOS_SDK)/lib_cxng/include \
		-I/tmp/bolos/arm-none-eabi/include \
		-Ibolos/bindgen/include \
		-target thumbv6-none-eabi \
		-mcpu=cortex-m0 -mthumb
else
generate:
	$(error invalid TARGET_NAME (not TARGET_NANOS or TARGET_NANOX))
	$(error TARGET_NAME = [$(TARGET_NAME)])
endif

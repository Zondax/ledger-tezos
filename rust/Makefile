.PHONY: test

ifdef BAKING
$(info *************** BAKING ENABLED *****************)
RUST_FEATURES=--features="baking"
BAKING=true
else
$(info ************* BAKING DISABLED ******************)
RUST_FEATURES=--features="wallet"
BAKING=false
endif

$(info BOLOS_SDK = [$(BOLOS_SDK)])

test:
	cargo test $(RUST_FEATURES),"dev"

test_vectors:
	cargo test $(RUST_FEATURES),"dev" vectors -- --nocapture

fuzz:
	@ cd hfuzz-parser && node scripts/gen_operations_corpus.js ../../zemu/test-vectors
	@ cd hfuzz-parser && node scripts/gen_operations_corpus.js ../../zemu/tests/data
	cd hfuzz-parser && HFUZZ_BUILD_ARGS=$(RUST_FEATURES) cargo hfuzz run operations

clean_fuzz:
	cd hfuzz-parser && cargo hfuzz clean

.PHONY: zui
zui:
	cd zemu && cbindgen --config cbindgen.toml --crate zemu-sys --output include/zemu_ui.h --lang c

.PHONY: prep_generate
prep_generate:
	- mkdir -p /tmp/bolos/arm-none-eabi
	@echo 'Please make sure `/tmp/bolos/arm-none-eabi` is populated correctly'
	@echo "To populate it: run the current builder-bolos image with the following command:"
	@echo -e 'docker run --rm \
	-v /tmp/bolos:/shared \
	DOCKER_IMAGE \
	"cp -r /opt/bolos/gcc-arm-none-eabi-10-2020-q4-major/arm-none-eabi/include /shared/arm-none-eabi/include"'
	@echo 'You should replace `DOCKER_IMAGE` with the current image and also the gcc archive path (after `/opt/bolos` and before `/arm-none-eabi`)'

ifeq ($(TARGET_NAME),TARGET_NANOX)
generate:
	- rm bolos-sys/src/bindings/bindingsX.rs
	bindgen --use-core \
			--with-derive-default \
			--no-derive-debug \
			--ctypes-prefix cty \
			-o bolos-sys/src/bindings/bindingsX.rs \
			bolos-sys/bindgen/wrapperX.h -- \
			-I$(BOLOS_SDK)/include \
			-I$(BOLOS_SDK)/lib_ux/include \
			-I$(BOLOS_SDK)/lib_cxng/include \
			-I/tmp/bolos/arm-none-eabi/include \
			-Ibolos-sys/bindgen/include \
			-target thumbv6-none-eabi \
			-mcpu=cortex-m0 -mthumb
	- rm zemu/include/zemu_ui_x.h
	RUSTUP_TOOLCHAIN=nightly cbindgen \
		--config zemu/cbindgen_x.toml \
		--crate zemu-sys \
		--output zemu/include/zemu_ui_x.h
else ifeq ($(TARGET_NAME),TARGET_NANOS)
generate:
	- rm bolos-sys/src/bindings/bindingsS.rs
	bindgen --use-core \
		--with-derive-default \
		---no-derive-debug \
		--ctypes-prefix cty \
		-o bolos-sys/src/bindings/bindingsS.rs \
		bolos-sys/bindgen/wrapperS.h -- \
		-I$(BOLOS_SDK)/include \
		-I$(BOLOS_SDK)/lib_ux/include \
		-I$(BOLOS_SDK)/lib_cxng/include \
		-I/tmp/bolos/arm-none-eabi/include \
		-Ibolos-sys/bindgen/include \
		-target thumbv6-none-eabi \
		-mcpu=cortex-m0 -mthumb
	- rm zemu/include/zemu_ui_s.h
	RUSTUP_TOOLCHAIN=nightly cbindgen \
		--config zemu/cbindgen_s.toml \
		--crate zemu-sys \
		--output zemu/include/zemu_ui_s.h
else ifeq ($(TARGET_NAME),TARGET_NANOS2)
generate:
	- rm bolos-sys/src/bindings/bindingsSP.rs
	bindgen --use-core \
			--with-derive-default \
			--no-derive-debug \
			--ctypes-prefix cty \
			-o bolos-sys/src/bindings/bindingsSP.rs \
			bolos-sys/bindgen/wrapperSP.h -- \
			-I$(BOLOS_SDK)/include \
			-I$(BOLOS_SDK)/lib_ux/include \
			-I$(BOLOS_SDK)/lib_cxng/include \
			-I/tmp/bolos/arm-none-eabi/include \
			-Ibolos-sys/bindgen/include \
			-target thumbv8m.main-none-eabi \
			-mcpu=cortex-m35p -mthumb
	- rm zemu/include/zemu_ui_sp.h
	RUSTUP_TOOLCHAIN=nightly cbindgen \
		--config zemu/cbindgen_sp.toml \
		--crate zemu-sys \
		--output zemu/include/zemu_ui_sp.h
else
generate:
	$(error invalid TARGET_NAME (not TARGET_NANOS or TARGET_NANOX or TARGET_NANOS2))
	$(error TARGET_NAME = [$(TARGET_NAME)])
endif

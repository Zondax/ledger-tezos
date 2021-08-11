#*******************************************************************************
#*   (c) 2019 Zondax GmbH
#*
#*  Licensed under the Apache License, Version 2.0 (the "License");
#*  you may not use this file except in compliance with the License.
#*  You may obtain a copy of the License at
#*
#*      http://www.apache.org/licenses/LICENSE-2.0
#*
#*  Unless required by applicable law or agreed to in writing, software
#*  distributed under the License is distributed on an "AS IS" BASIS,
#*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#*  See the License for the specific language governing permissions and
#*  limitations under the License.
#********************************************************************************

# We use BOLOS_SDK to determine the development environment that is being used
# BOLOS_SDK IS  DEFINED	 	We use the plain Makefile for Ledger
# BOLOS_SDK NOT DEFINED		We use a containerized build approach

TESTS_JS_PACKAGE = "@zondax/ledger-tezos"
TESTS_JS_DIR = $(CURDIR)/js

DOCKER_LEGACY_APP_SRC=/project/legacy
DOCKER_LEGACY_APP_BIN=$(DOCKER_LEGACY_APP_SRC)/bin/app.elf

ifeq ($(BOLOS_SDK),)
	# TODO: use earthly here
	include $(CURDIR)/rust/app/refactor/dockerized_build.mk

lint:
	cd rust && cargo fmt

both:
	$(MAKE)
	BAKING=tezos_baking $(MAKE)

.PHONY: legacy legacy_wallet legacy_baking legacy_impl
legacy:
	$(MAKE) clean_legacy
	$(MAKE) legacy_baking
	$(MAKE) clean_legacy
	$(MAKE) legacy_wallet

legacy_impl:
	$(call run_docker,$(DOCKER_BOLOS_SDKS),make -j $(NPROC) -C $(DOCKER_LEGACY_APP_SRC))

legacy_wallet:
	- mkdir -p legacy/output || true
	BAKING=tezos_wallet $(MAKE) legacy_impl
	mv legacy/bin/app.elf legacy/output/app.elf

legacy_baking:
	- mkdir -p legacy/output || true
	BAKING=tezos_baking $(MAKE) legacy_impl
	mv legacy/bin/app.elf legacy/output/app_baking.elf

.PHONY: clean_legacy
clean_legacy:
	$(call run_docker,$(DOCKER_BOLOS_SDKS), make -C $(DOCKER_LEGACY_APP_SRC) clean)

test_all:
	make rust_test
	make zemu_install
	make clean_build
	BAKING=yes make
	make
	make legacy
	make zemu_test

else
default:
	$(MAKE) -C rust/app

generate:
	$(MAKE) -C rust generate

both:
	$(MAKE)
	BAKING=tezos_baking $(MAKE)

.PHONY: legacy
legacy:
	- mkdir -p legacy/output || true
	$(MAKE) -C legacy clean
	APP=tezos_wallet $(MAKE) -C legacy
	$(MAKE) -C legacy clean
	APP=tezos_baking $(MAKE) -C legacy
%:
	$(info "Calling app Makefile for target $@")
	COIN=$(COIN) $(MAKE) -C rust/app $@
endif

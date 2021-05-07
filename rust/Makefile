.PHONY: test

ifdef BAKING
$(info *************** BAKING ENABLED *****************)
RUST_FEATURES=--features="baking"
else
$(info ************* BAKING DISABLED ******************)
RUST_FEATURES=--features="wallet"
endif

test:
	cargo test $(RUST_FEATURES),"dev"

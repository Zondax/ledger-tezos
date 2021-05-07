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

test:
	cargo test $(RUST_FEATURES),"dev"

.PHONY: check ci clippy fmt install lint publish release test

BIN_NAME = link_auditor
CARGO = $(shell which cargo)

check:
	$(CARGO) check --release

ci: lint check test

clippy:
	$(CARGO) clippy

fmt:
	@$(CARGO) fmt

install:
	cp ./target/release/$(BIN_NAME) /usr/local/bin/$(BIN_NAME)

lint:
	$(CARGO) fmt --all -- --check

publish:
	$(CARGO) publish

release:
	@$(CARGO) build --release

test:
	@$(CARGO) test -- --nocapture

.PHONY: check ci clippy fmt install lint publish release test

BIN_NAME = link_auditor
CARGO = $(shell which cargo)

build:
	@$(CARGO) build

check:
	$(CARGO) check --release

ci: lint check test

clippy:
	$(CARGO) clippy

fmt:
	@$(CARGO) fmt

install:
	cp -f ./target/release/$(BIN_NAME) /usr/local/bin/$(BIN_NAME)

link:
	@ln -sf ./target/debug/$(BIN_NAME) .

lint:
	$(CARGO) fmt --all -- --check

publish:
	$(CARGO) publish

release:
	@$(CARGO) build --release

test:
	@$(CARGO) test -- --nocapture

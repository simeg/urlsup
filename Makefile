.PHONY: check ci clippy dockerfile fmt install lint publish publish-dockerfile release test

BIN_NAME  = urlsup
VERSION   = $(shell awk -F'[ ="]+' '$$1 == "version" { print $$2 }' ./Cargo.toml)
CARGO     = $(shell which cargo)
DOCKER    = $(shell which docker)

build:
	@$(CARGO) build

ci: lint clippy build test

clippy:
	$(CARGO) clippy

dockerfile:
	$(DOCKER) build -t simeg/urlsup:latest -t simeg/urlsup:$(VERSION) - < Dockerfile

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

publish-dockerfile: dockerfile
	$(DOCKER) push simeg/urlsup

release:
	@$(CARGO) build --release

test:
	@$(CARGO) test -- --nocapture

.PHONY: build check ci clippy fmt link lint publish release test

BIN_NAME  = urlsup
CARGO     = $(shell which cargo)

build:
	@$(CARGO) build

check:
	@$(CARGO) build --check

ci: lint clippy test

clippy:
	$(CARGO) clippy

fmt:
	@$(CARGO) fmt

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

.PHONY: check ci clippy fmt link lint publish release test

BIN_NAME  = urlsup
CARGO     = $(shell which cargo)

check:
	@$(CARGO) build --check

ci: lint clippy build test

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

.PHONY: audit build check ci clippy coverage fmt link lint publish release test

BIN_NAME  = urlsup
CARGO     = $(shell which cargo)

build:
	@$(CARGO) build

check:
	@$(CARGO) build --check

audit:
	@$(CARGO) audit --deny warnings

ci: lint clippy test

clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

coverage:
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "Installing cargo-tarpaulin..."; \
		$(CARGO) install cargo-tarpaulin; \
	}
	$(CARGO) tarpaulin --all-features --workspace --timeout 120 --out html --output-dir coverage
	@echo "Coverage report generated in coverage/tarpaulin-report.html"

fmt:
	@$(CARGO) fmt
	@$(CARGO) clippy --all-targets --all-features --fix --allow-dirty

link:
	@ln -sf ./target/debug/$(BIN_NAME) .

lint:
	$(CARGO) fmt --all -- --check

publish: ci
	$(CARGO) publish

release:
	@$(CARGO) build --release

test:
	@$(CARGO) test -- --nocapture

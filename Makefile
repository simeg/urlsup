.PHONY: audit build check ci clippy coverage fmt generate_test_links help install lint publish release test

BIN_NAME  = urlsup
CARGO     = $(shell which cargo)

help: ## Show this help message
	@echo "urlsup - A fast, async URL validator for documentation and CI pipelines"
	@echo ""
	@echo "Available commands:"
	@echo ""
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Examples:"
	@echo "  make build          # Build debug version"
	@echo "  make release        # Build optimized release version"
	@echo "  make test           # Run all tests"
	@echo "  make ci             # Run all CI checks"
	@echo "  make install        # Install to cargo bin directory"
	@echo ""
	@echo "For more information, see: https://github.com/segersand/urlsup"

audit: ## Run security audit on dependencies
	@$(CARGO) audit --deny warnings

build: ## Build debug version of the binary
	@$(CARGO) build

check: ## Check if the project compiles (in release mode) without building
	@$(CARGO) check --release --all-targets

ci: check lint clippy test ## Run all CI checks (lint, clippy, test)

clippy: ## Run clippy linter with strict warnings
	$(CARGO) clippy --all-targets --all-features -- -D warnings

coverage: ## Generate test coverage report (requires cargo-tarpaulin)
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "Installing cargo-tarpaulin..."; \
		$(CARGO) install cargo-tarpaulin; \
	}
	$(CARGO) tarpaulin --all-features --workspace --timeout 120 --out html --output-dir coverage
	@echo "Coverage report generated in coverage/tarpaulin-report.html"

fmt: ## Format code and auto-fix clippy issues
	@$(CARGO) fmt
	@$(CARGO) clippy --all-targets --all-features --fix --allow-dirty

generate_test_links: ## Generate test directory structure with sample URLs
	@python3 ./scripts/generate_test_links.py

install: ## Install urlsup to cargo bin directory
	@$(CARGO) install --path .

lint: ## Check code formatting
	@$(CARGO) fmt --all -- --check

publish: ci ## Publish to crates.io (after running CI checks)
	@$(CARGO) publish

release: ## Build optimized release version
	@$(CARGO) build --release

test: ## Run all tests with output
	@RUST_LOG=error,urlsup=debug $(CARGO) test -- --nocapture

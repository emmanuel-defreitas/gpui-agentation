.DEFAULT_GOAL := help
.DELETE_ON_ERROR:

CARGO ?= cargo

.PHONY: help fetch consumer bridge bridge-check build check fmt fmt-check lint test test-doc doc release-check version-check version-test release-prepare release-version-check ci

help: ## Show available commands
	@awk 'BEGIN { FS = ":.*## "; print "Usage: make <target>\n" } /^[a-zA-Z0-9_-]+:.*## / { printf "  %-18s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

fetch: ## Fetch locked registry dependencies
	$(CARGO) fetch --locked

consumer: ## Run the complete consumer app with minimal integration
	$(CARGO) run -p agentation-consumer --locked

bridge: ## Build the portable stdio MCP executable without native UI
	$(CARGO) build --release --bin gpui-agentation --locked

bridge-check: ## Verify MCP-only compilation and tests
	$(CARGO) test --no-default-features --features mcp --locked
	$(CARGO) check --release --no-default-features --features mcp --locked

build: ## Build the workspace
	$(CARGO) build --workspace --all-targets --all-features --locked

check: ## Type-check the workspace
	$(CARGO) check --workspace --all-targets --all-features --locked

fmt: ## Format all Rust sources
	$(CARGO) fmt --all

fmt-check: ## Check Rust formatting without editing files
	$(CARGO) fmt --all -- --check

lint: ## Run Clippy with warnings treated as errors
	$(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

test: ## Run workspace tests, including documentation tests
	$(CARGO) test --workspace --all-features --locked

test-doc: ## Run only documentation tests
	$(CARGO) test --workspace --all-features --locked --doc

doc: ## Generate development API documentation without dependencies
	$(CARGO) doc --workspace --all-features --no-deps --locked

release-check: ## Check compilation with the annotation API disabled
	$(CARGO) check --workspace --all-targets --release --all-features --locked

version-check: ## Verify documented crate versions match Cargo.toml and server.json
	PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-version-refs.py
	PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-bridge-release.py

version-test: ## Test release synchronization and conventional-commit version rules
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s .github/scripts -p 'test_release*.py'

release-prepare: ## Synchronize the next release version locally (optional VERSION=x.y.z)
	PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/release_version.py prepare "$(VERSION)"

release-version-check: ## Verify metadata and the version required for promotion
	PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/release_version.py check-release

ci: ## Run the same formatting, lint, test, and release checks as Rust CI
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE)
	$(MAKE) version-check
	$(MAKE) version-test
	$(MAKE) release-check
	$(MAKE) bridge-check

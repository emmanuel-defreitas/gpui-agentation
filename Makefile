.DEFAULT_GOAL := help
.DELETE_ON_ERROR:

CARGO ?= cargo

.PHONY: help fetch dev consumer bridge bridge-check build check fmt fmt-check lint test test-doc doc release-check hot-reload-test version-check ci workflows-check

help: ## Show available commands
	@awk 'BEGIN { FS = ":.*## "; print "Usage: make <target>\n" } /^[a-zA-Z0-9_-]+:.*## / { printf "  %-18s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

fetch: ## Fetch locked registry dependencies
	$(CARGO) fetch --locked

dev: ## Run the demo with the Settings hot-reload toggle available
	CARGO="$(CARGO)" python3 src/hot_reload/runner.py --manifest-path Cargo.toml --example demo --no-default-features --features native

consumer: ## Run the complete consumer app with minimal integration
	$(CARGO) run -p agentation-consumer --locked

bridge: ## Build the portable stdio MCP executable without native UI
	$(CARGO) build --release --bin gpui-agentation --locked

bridge-check: ## Verify MCP-only compilation and tests
	$(CARGO) test --no-default-features --features mcp --locked
	$(CARGO) check --release --no-default-features --features mcp --locked

build: ## Build the workspace and demo
	$(CARGO) build --workspace --all-targets --all-features --locked

check: ## Type-check the workspace and demo
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

# Keep these sequential, including under make -j, and stop on the first failure.
hot-reload-test: ## Test watcher filtering, failed builds, restart recovery and cleanup
	PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s src/hot_reload -p 'test_*.py'

version-check: ## Verify documented crate versions match Cargo.toml and server.json
	python3 .github/scripts/check-version-refs.py
	python3 .github/scripts/check-bridge-release.py

ci: ## Run the same formatting, lint, test, and release checks as Rust CI
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) hot-reload-test
	$(MAKE) version-check
	$(MAKE) release-check
	$(MAKE) bridge-check

workflows-check: ## Compile and lint agentic workflows (requires gh aw and actionlint)
	gh aw compile --actionlint

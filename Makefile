.DEFAULT_GOAL := help
.DELETE_ON_ERROR:

CARGO ?= cargo

.PHONY: help fetch example example-stock test-external bridge bridge-check bridge-install-check build check fmt fmt-check lint test test-doc doc release-check version-check version-test release-prepare release-version-check platform-check snapshot-check site-check verify ci

help: ## Show available commands
	@awk 'BEGIN { FS = ":.*## "; print "Usage: make <target>\n" } /^[a-zA-Z0-9_-]+:.*## / { printf "  %-18s %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

fetch: ## Fetch locked registry dependencies
	$(CARGO) fetch --locked

example: fetch ## Run the example app with the inspector outside the app (patched GPUI)
	python3 scripts/inspector-poc.py run -p agentation-consumer

example-stock: ## Run the example app on stock GPUI (inspector draws inside the app)
	$(CARGO) run -p agentation-consumer --locked

test-external: fetch ## Run the lib tests with the external inspector enabled
	python3 scripts/inspector-poc.py test --lib

bridge: ## Build the portable stdio MCP executable without native UI
	$(CARGO) build --release --bin gpui-agentation --locked

bridge-check: ## Verify MCP-only compilation and tests
	$(CARGO) clippy --all-targets --locked -- -D warnings
	$(CARGO) test --no-default-features --features mcp --locked
	$(CARGO) build --release --bin gpui-agentation --locked

bridge-install-check: ## Smoke-test the default Cargo install and executable locally
	@set -eu; install_dir=$$(mktemp -d); trap 'rm -rf "$$install_dir"' EXIT; \
		$(CARGO) install --path . --locked --debug --root "$$install_dir"; \
		"$$install_dir/bin/gpui-agentation" --version

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

platform-check: ## Require macOS for the supported native library and local checks
	@test "$$(uname -s)" = Darwin || { echo 'Native development and verification require macOS.' >&2; exit 1; }

snapshot-check: ## Verify the pinned bundled Ely snapshot
	PYTHONDONTWRITEBYTECODE=1 python3 .github/scripts/check-ely-snapshot.py

site-check: ## Install website dependencies and validate the website locally
	cd site && npm ci && npm run check

verify: ## Run the complete local macOS verification suite before merging or promotion
	$(MAKE) platform-check
	$(MAKE) snapshot-check
	$(MAKE) fmt-check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) version-check
	$(MAKE) version-test
	$(MAKE) release-check
	$(MAKE) bridge-check
	$(MAKE) bridge-install-check

ci: verify ## Alias for local verification; GitHub Actions only promotes and releases

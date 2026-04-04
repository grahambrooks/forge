.PHONY: build test check lint fmt clean pre-commit update help integration-test integration-test-local

CARGO := cargo
SOURCE := forge/examples/payments.forge

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# ── Build ──────────────────────────────────────────────────────────

build: ## Build the forge binary (debug)
	cd forge && $(CARGO) build

release: ## Build the forge binary (release, optimized)
	cd forge && $(CARGO) build --release

# ── Test ───────────────────────────────────────────────────────────

test: ## Run all tests
	cd forge && $(CARGO) test

test-verbose: ## Run all tests with output
	cd forge && $(CARGO) test -- --nocapture

# ── Lint & Format ──────────────────────────────────────────────────

lint: ## Run clippy lints
	cd forge && $(CARGO) clippy -- -D warnings

fmt: ## Format all Rust code
	cd forge && $(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	cd forge && $(CARGO) fmt -- --check

check: ## Run forge check (architectural linting) on the example
	cd forge && $(CARGO) run -- check --source examples/payments.forge --severity info

# ── Pre-commit ─────────────────────────────────────────────────────

pre-commit: fmt-check lint test ## Run all pre-commit checks (format, lint, test)
	@echo "All pre-commit checks passed."

# ── Dependencies ───────────────────────────────────────────────────

update: ## Update Cargo dependencies
	cd forge && $(CARGO) update

outdated: ## Show outdated dependencies (requires cargo-outdated)
	cd forge && $(CARGO) outdated

# ── Clean ──────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cd forge && $(CARGO) clean

# ── Convenience ────────────────────────────────────────────────────

# ── Integration Tests ─────────────────────────────────────────────

integration-test: ## Run integration tests in Docker
	docker compose -f tests/integration/docker-compose.yml up --build --exit-code-from integration-test
	docker compose -f tests/integration/docker-compose.yml down

integration-test-local: release ## Run integration tests using local binary (no Docker)
	FORGE=./forge/target/release/forge FIXTURES=./tests/integration/fixtures ./tests/integration/run-tests.sh

# ── Convenience ────────────────────────────────────────────────────

run: build ## Build and render all views from the example
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output --style outline

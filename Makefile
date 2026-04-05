.PHONY: build test check lint fmt clean pre-commit update help integration-test integration-test-local release-docker release-github install-test install-test-debian install-test-ubuntu install-test-fedora install-test-alpine install-test-homebrew install-test-cargo

CARGO := cargo
SOURCE := forge/examples/payments.forge

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

# ── Build ──────────────────────────────────────────────────────────

build: ## Build the forge binary (debug)
	cd forge && $(CARGO) build

release: ## Build the forge binary (release, optimized)
	cd forge && $(CARGO) build --release

release-docker: ## Build Linux release binaries (glibc + musl) in Docker → dist/
	docker build -f dist/Dockerfile --output type=local,dest=dist .
	@echo "Built: dist/forge-gnu (glibc), dist/forge-musl (static musl)"

release-github: pre-commit ## Tag and push a calver release (vYYYY.MM.DD) to trigger GitHub Actions release
	@VERSION=$$(date +%Y.%m.%d) && \
	CARGO_VERSION=$$(date +%Y.%-m.%-d) && \
	TAG="v$$VERSION" && \
	if git rev-parse "$$TAG" >/dev/null 2>&1; then \
		echo "Error: tag $$TAG already exists"; exit 1; \
	fi && \
	echo "Releasing $$TAG (version $$VERSION) ..." && \
	sed -i '' "s/^version = \".*\"/version = \"$$CARGO_VERSION\"/" forge/Cargo.toml && \
	sed -i '' "s/^  version \".*\"/  version \"$$VERSION\"/" Formula/forge.rb && \
	sed -i '' "s|/download/v[^/]*/|/download/$$TAG/|g" Formula/forge.rb && \
	cd forge && $(CARGO) build --release --quiet && cd .. && \
	BUILT_VERSION=$$(./forge/target/release/forge --version 2>&1 | awk '{print $$2}') && \
	if [ "$$BUILT_VERSION" != "$$VERSION" ]; then \
		echo "Error: binary reports version '$$BUILT_VERSION' but expected '$$VERSION'"; exit 1; \
	fi && \
	echo "Verified: binary version matches $$VERSION" && \
	git add forge/Cargo.toml forge/Cargo.lock Formula/forge.rb && \
	git commit -m "Release $$TAG" && \
	git tag -a "$$TAG" -m "Release $$TAG" && \
	git push origin main "$$TAG" && \
	echo "Pushed $$TAG — GitHub Actions will build and publish the release"

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
	rm -rf dist/forge-gnu dist/forge-musl

# ── Convenience ────────────────────────────────────────────────────

# ── Integration Tests ─────────────────────────────────────────────

integration-test: ## Run integration tests in Docker
	docker compose -f tests/integration/docker-compose.yml up --build --exit-code-from integration-test
	docker compose -f tests/integration/docker-compose.yml down

integration-test-local: release ## Run integration tests using local binary (no Docker)
	FORGE=./forge/target/release/forge FIXTURES=./tests/integration/fixtures ./tests/integration/run-tests.sh

install-test: release-docker ## Run installation tests on multiple distros (Debian, Ubuntu, Fedora, Alpine)
	docker compose -f tests/integration/install/docker-compose.yml up --build
	docker compose -f tests/integration/install/docker-compose.yml down

install-test-debian: release-docker ## Run installation test on Debian
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-debian
	docker compose -f tests/integration/install/docker-compose.yml down install-test-debian

install-test-ubuntu: release-docker ## Run installation test on Ubuntu
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-ubuntu
	docker compose -f tests/integration/install/docker-compose.yml down install-test-ubuntu

install-test-fedora: release-docker ## Run installation test on Fedora
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-fedora
	docker compose -f tests/integration/install/docker-compose.yml down install-test-fedora

install-test-alpine: release-docker ## Run installation test on Alpine (static musl build)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-alpine
	docker compose -f tests/integration/install/docker-compose.yml down install-test-alpine

install-test-homebrew: release-docker ## Run installation test via Homebrew (Linuxbrew + local tap)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-homebrew
	docker compose -f tests/integration/install/docker-compose.yml down install-test-homebrew

install-test-cargo: ## Run installation test via cargo install (builds from source)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-cargo
	docker compose -f tests/integration/install/docker-compose.yml down install-test-cargo

# ── Convenience ────────────────────────────────────────────────────

run: build ## Build and render all views from the example
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output --style outline

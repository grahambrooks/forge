.PHONY: help \
	build build-release release-docker publish-check \
	test test-verbose \
	lint fmt fmt-check check pre-commit hooks \
	run forge-check \
	eval eval-smoke eval-all eval-claude eval-compare eval-site eval-report eval-clean \
	integration-test integration-test-local \
	install-test install-test-debian install-test-ubuntu install-test-fedora \
	install-test-alpine install-test-homebrew install-test-cargo \
	update outdated clean watch-ci

CARGO         := cargo
SOURCE        := forge/examples/payments.forge
# Build into the repo, whatever a user's cargo config says: the targets that
# run the binary (evals, integration tests) need to know where it is. An
# explicit CARGO_TARGET_DIR still wins.
export CARGO_TARGET_DIR ?= $(CURDIR)/forge/target
RELEASE_FORGE := $(CARGO_TARGET_DIR)/release/forge

##@ General

help: ## Show this help (grouped by section)
	@awk 'BEGIN { FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n" } \
		/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5); next } \
		/^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2 }' \
		$(MAKEFILE_LIST)

##@ Build

build: ## Build the forge binary (debug)
	cd forge && $(CARGO) build

build-release: ## Build the forge binary (release, optimized)
	cd forge && $(CARGO) build --release

release-docker: ## Build Linux release binaries (glibc + musl) in Docker → dist/
	docker build -f dist/Dockerfile --output type=local,dest=dist .
	@echo "Built: dist/forge-gnu (glibc), dist/forge-musl (static musl)"

publish-check: ## Dry-run the crates.io package step (what the release workflow publishes)
	cd forge && $(CARGO) publish --locked --dry-run

##@ Test

test: ## Run all unit + integration tests, as CI does
	cd forge && $(CARGO) test --workspace --all-features --locked

test-verbose: ## Run all tests with stdout/stderr streamed
	cd forge && $(CARGO) test -- --nocapture

##@ Lint & Format

lint: ## Run clippy with warnings-as-errors, as CI does
	cd forge && $(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

fmt: ## Format all Rust code
	cd forge && $(CARGO) fmt --all

fmt-check: ## Check formatting without modifying files
	cd forge && $(CARGO) fmt --all --check

check: fmt-check lint test ## The pre-push gate: exactly the CI check job (fmt, clippy, test)
	@echo "All pre-commit checks passed."

pre-commit: check ## Alias for check

hooks: ## Install the prek git hooks (pre-commit and pre-push)
	prek install

##@ Run & Demo

forge-check: ## Run `forge check` (architectural linting) on the bundled example
	cd forge && $(CARGO) run -- check --source examples/payments.forge --severity info

run: build ## Build and render every view from the bundled example
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output --style outline

##@ Evaluation

# All eval targets use the release binary so cold starts aren't noise.
# See eval/README.md for the corpus definition and scoring model.

eval: build-release ## Run the baseline evaluation driver (tier 1+2, ~15 min)
	cd eval && ./run.py --forge $(RELEASE_FORGE)

eval-smoke: build-release ## Run the tier-1 smoke evaluation (~2 min, suitable as a CI gate)
	cd eval && ./run.py --tier 1 --forge $(RELEASE_FORGE)

eval-all: build-release ## Run every tier (1+2+3) including stretch-tier monorepos
	cd eval && ./run.py --tier all --forge $(RELEASE_FORGE)

eval-claude: build-release ## Run the Claude-driven companion evaluation (tier 1, billed)
	cd eval && ./run_claude.py --forge $(RELEASE_FORGE)

eval-compare: ## Diff baseline vs Claude results into results-claude/compare.md
	cd eval && ./compare.py

eval-site: ## Regenerate the static comparison site from cached results/
	cd eval && ./sitegen.py

eval-report: ## Regenerate results/report.md from cached results/
	cd eval && ./run.py report

eval-clean: ## Remove eval work/, results/, and results-claude/
	cd eval && ./run.py clean
	cd eval && ./run_claude.py clean

##@ Integration Tests

integration-test: ## Run integration tests in Docker
	docker compose -f tests/integration/docker-compose.yml up --build --exit-code-from integration-test
	docker compose -f tests/integration/docker-compose.yml down

integration-test-local: build-release ## Run integration tests using the local release binary (no Docker)
	FORGE=$(RELEASE_FORGE) FIXTURES=./tests/integration/fixtures ./tests/integration/run-tests.sh

##@ Install Tests (Docker)

install-test: release-docker ## Run install tests on every distro (Debian, Ubuntu, Fedora, Alpine)
	docker compose -f tests/integration/install/docker-compose.yml up --build
	docker compose -f tests/integration/install/docker-compose.yml down

install-test-debian: release-docker ## Run install test on Debian
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-debian
	docker compose -f tests/integration/install/docker-compose.yml down install-test-debian

install-test-ubuntu: release-docker ## Run install test on Ubuntu
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-ubuntu
	docker compose -f tests/integration/install/docker-compose.yml down install-test-ubuntu

install-test-fedora: release-docker ## Run install test on Fedora
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-fedora
	docker compose -f tests/integration/install/docker-compose.yml down install-test-fedora

install-test-alpine: release-docker ## Run install test on Alpine (static musl build)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-alpine
	docker compose -f tests/integration/install/docker-compose.yml down install-test-alpine

install-test-homebrew: release-docker ## Run install test via Homebrew (Linuxbrew + local tap)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-homebrew
	docker compose -f tests/integration/install/docker-compose.yml down install-test-homebrew

install-test-cargo: ## Run install test via `cargo install` (builds from source)
	docker compose -f tests/integration/install/docker-compose.yml up --build install-test-cargo
	docker compose -f tests/integration/install/docker-compose.yml down install-test-cargo

##@ Dependencies

update: ## Update Cargo dependencies
	cd forge && $(CARGO) update

outdated: ## Show outdated dependencies (requires cargo-outdated)
	cd forge && $(CARGO) outdated

##@ Clean

clean: ## Remove Rust build artifacts and Docker release outputs
	cd forge && $(CARGO) clean
	rm -rf dist/forge-gnu dist/forge-musl

##@ CI

watch-ci: ## After pushing, wait for every CI run on HEAD; fails if any run fails
	@command -v watch-ci >/dev/null || { echo "watch-ci not on PATH: ln -s ~/dev/projects/agentic-twin/scripts/watch-ci.sh ~/.local/bin/watch-ci" >&2; exit 1; }
	watch-ci

##@ Release

# `make release` / `release-dry` / `release-version`: tag origin/main with the next
# CalVer and push the tag; .github/workflows/release.yml does the rest.
include release.mk

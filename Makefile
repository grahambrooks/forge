.PHONY: help \
	build release release-docker release-github \
	test test-verbose \
	lint fmt fmt-check check pre-commit \
	run \
	eval eval-smoke eval-all eval-claude eval-compare eval-site eval-report eval-clean \
	integration-test integration-test-local \
	install-test install-test-debian install-test-ubuntu install-test-fedora \
	install-test-alpine install-test-homebrew install-test-cargo \
	update outdated clean

CARGO         := cargo
SOURCE        := forge/examples/payments.forge
RELEASE_FORGE := forge/target/release/forge

##@ General

help: ## Show this help (grouped by section)
	@awk 'BEGIN { FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n" } \
		/^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5); next } \
		/^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2 }' \
		$(MAKEFILE_LIST)

##@ Build

build: ## Build the forge binary (debug)
	cd forge && $(CARGO) build

release: ## Build the forge binary (release, optimized)
	cd forge && $(CARGO) build --release

release-docker: ## Build Linux release binaries (glibc + musl) in Docker → dist/
	docker build -f dist/Dockerfile --output type=local,dest=dist .
	@echo "Built: dist/forge-gnu (glibc), dist/forge-musl (static musl)"

release-github: pre-commit ## Tag and push a calver release (vYYYY.MM.DD) to trigger GitHub Actions release
	@bash -ec '\
	VERSION=$$(date +%Y.%m.%d); \
	CARGO_VERSION=$$(date +%Y.%-m.%-d); \
	TAG="v$$VERSION"; \
	echo "Releasing $$TAG (version $$VERSION) ..."; \
	sed -i "" "s/^version = \".*\"/version = \"$$CARGO_VERSION\"/" forge/Cargo.toml; \
	cd forge && cargo build --release --quiet && cd ..; \
	BUILT_VERSION=$$(./forge/target/release/forge --version 2>&1 | awk "{print \$$2}"); \
	if [ "$$BUILT_VERSION" != "$$VERSION" ]; then \
		echo "Error: binary reports version $$BUILT_VERSION but expected $$VERSION"; \
		exit 1; \
	fi; \
	echo "Verified: binary version matches $$VERSION"; \
	git add forge/Cargo.toml forge/Cargo.lock; \
	git diff --cached --quiet || git commit -m "Release $$TAG"; \
	if git rev-parse "$$TAG" >/dev/null 2>&1; then \
		echo "Updating existing tag $$TAG ..."; \
		gh release delete "$$TAG" --yes 2>/dev/null || true; \
		git tag -d "$$TAG"; \
		git push origin ":refs/tags/$$TAG"; \
	fi; \
	git tag -a "$$TAG" -m "Release $$TAG"; \
	git push origin main "$$TAG"; \
	echo "Pushed $$TAG — GitHub Actions will build and publish the release"; \
	echo "The homebrew formula will be updated automatically after the release is published"; \
	'

##@ Test

test: ## Run all unit + integration tests
	cd forge && $(CARGO) test

test-verbose: ## Run all tests with stdout/stderr streamed
	cd forge && $(CARGO) test -- --nocapture

##@ Lint & Format

lint: ## Run clippy with warnings-as-errors
	cd forge && $(CARGO) clippy -- -D warnings

fmt: ## Format all Rust code
	cd forge && $(CARGO) fmt

fmt-check: ## Check formatting without modifying files
	cd forge && $(CARGO) fmt -- --check

check: ## Run `forge check` (architectural linting) on the bundled example
	cd forge && $(CARGO) run -- check --source examples/payments.forge --severity info

pre-commit: fmt-check lint test ## Run fmt-check + lint + test (CI-equivalent gate)
	@echo "All pre-commit checks passed."

##@ Run & Demo

run: build ## Build and render every view from the bundled example
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output
	cd forge && $(CARGO) run -- build --source examples/payments.forge --out output --style outline

##@ Evaluation

# All eval targets use the release binary so cold starts aren't noise.
# See eval/README.md for the corpus definition and scoring model.

eval: release ## Run the baseline evaluation driver (tier 1+2, ~15 min)
	cd eval && ./run.py --forge ../$(RELEASE_FORGE)

eval-smoke: release ## Run the tier-1 smoke evaluation (~2 min, suitable as a CI gate)
	cd eval && ./run.py --tier 1 --forge ../$(RELEASE_FORGE)

eval-all: release ## Run every tier (1+2+3) including stretch-tier monorepos
	cd eval && ./run.py --tier all --forge ../$(RELEASE_FORGE)

eval-claude: release ## Run the Claude-driven companion evaluation (tier 1, billed)
	cd eval && ./run_claude.py --forge ../$(RELEASE_FORGE)

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

integration-test-local: release ## Run integration tests using the local release binary (no Docker)
	FORGE=./forge/target/release/forge FIXTURES=./tests/integration/fixtures ./tests/integration/run-tests.sh

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

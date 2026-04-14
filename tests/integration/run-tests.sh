#!/usr/bin/env bash
# Integration tests for forge analyze → build → generate → check → export pipeline.
# Runs against fixture projects that exercise each scanner.
#
# Usage:
#   In Docker:  docker compose -f tests/integration/docker-compose.yml up --build
#   Locally:    FORGE=./forge/target/release/forge FIXTURES=./tests/integration/fixtures ./tests/integration/run-tests.sh
set -euo pipefail

FORGE="${FORGE:-/usr/local/bin/forge}"
FIXTURES="${FIXTURES:-/fixtures}"
RESULTS="${RESULTS_DIR:-/tmp/forge-integration}"
FAILURES=0
PASSES=0

# ── Helpers ──────────────────────────────────────────────────────

pass() { PASSES=$((PASSES + 1)); echo "  PASS: $1"; }
fail() { FAILURES=$((FAILURES + 1)); echo "  FAIL: $1"; }

check_file_exists() {
    if [ -f "$1" ]; then pass "$2"; else fail "$2 (missing: $1)"; fi
}

check_file_not_empty() {
    if [ -s "$1" ]; then pass "$2"; else fail "$2 (empty: $1)"; fi
}

check_contains() {
    if grep -q "$2" "$1" 2>/dev/null; then pass "$3"; else fail "$3 (expected '$2' in $(basename "$1"))"; fi
}

check_count_gt() {
    local count
    count=$(find "$1" -name "$2" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$count" -gt 0 ]; then pass "$3 ($count found)"; else fail "$3 (none found in $1)"; fi
}

# ── Test runner ──────────────────────────────────────────────────

run_fixture() {
    local name=$1
    local dir="$FIXTURES/$name"
    local out="$RESULTS/$name"
    shift
    # remaining args are scanner list override
    local scanners="${1:-code,semantic,ci,docker,k8s,infra}"

    echo ""
    echo "=== $name ==="
    rm -rf "$out"
    mkdir -p "$out"

    # Step 1: analyze
    if $FORGE analyze "$dir" --out "$out/model.forge" --scanners "$scanners" 2>&1; then
        check_file_not_empty "$out/model.forge" "$name: analyze produced .forge"
    else
        fail "$name: analyze command failed"
        return
    fi

    # Step 2: build (may produce no views if model has no view definitions — that's ok)
    mkdir -p "$out/svg"
    $FORGE build --source "$out/model.forge" --out "$out/svg" 2>&1 || true

    # Step 3: generate
    mkdir -p "$out/site"
    $FORGE generate --source "$out/model.forge" --out "$out/site" 2>&1 || true

    # Step 4: check
    $FORGE check --source "$out/model.forge" --severity info 2>&1 || true

    # Step 5: export JSON
    $FORGE export --source "$out/model.forge" -f json > "$out/model.json" 2>&1
    check_file_not_empty "$out/model.json" "$name: export produced JSON"
}

# ── Rust / Axum ──────────────────────────────────────────────────
# code scanner detects framework from Cargo.toml; semantic scanner finds routes
# (routes go to api_catalogs which emitter doesn't serialize to .forge yet)

run_fixture "rust-axum" "code,semantic,docker"
check_contains "$RESULTS/rust-axum/model.forge" "Rust / Axum"   "rust-axum: detected Axum framework"
check_contains "$RESULTS/rust-axum/model.forge" "payment-api"    "rust-axum: detected crate name"
check_contains "$RESULTS/rust-axum/model.forge" "PostgreSQL"     "rust-axum: detected postgres connection"
check_contains "$RESULTS/rust-axum/model.forge" "docker"         "rust-axum: detected Dockerfile"

# ── Node.js / Express ────────────────────────────────────────────

run_fixture "node-express" "code,semantic"
check_contains "$RESULTS/node-express/model.forge" "TypeScript / Express" "node-express: detected Express+TS"
check_contains "$RESULTS/node-express/model.forge" "user-service"         "node-express: detected package name"
check_contains "$RESULTS/node-express/model.forge" "PostgreSQL"           "node-express: detected postgres connection"

# ── Go / Gin ─────────────────────────────────────────────────────

run_fixture "go-gin" "code,semantic"
check_contains "$RESULTS/go-gin/model.forge" "Go / Gin"         "go-gin: detected Gin framework"
check_contains "$RESULTS/go-gin/model.forge" "inventory-svc"    "go-gin: detected module name"

# ── Python / FastAPI ─────────────────────────────────────────────

run_fixture "python-fastapi" "code,semantic"
check_contains "$RESULTS/python-fastapi/model.forge" "Python / FastAPI" "python-fastapi: detected FastAPI"
check_contains "$RESULTS/python-fastapi/model.forge" "order-service"    "python-fastapi: detected project name"
check_contains "$RESULTS/python-fastapi/model.forge" "PostgreSQL"       "python-fastapi: detected postgres connection"

# ── Java / Spring Boot ───────────────────────────────────────────

run_fixture "java-spring" "code,semantic"
check_contains "$RESULTS/java-spring/model.forge" "Spring Boot"     "java-spring: detected Spring Boot"

# ── Docker Compose ───────────────────────────────────────────────

run_fixture "docker-compose" "docker"
check_contains "$RESULTS/docker-compose/model.forge" "PostgreSQL"    "docker-compose: detected postgres service"
check_contains "$RESULTS/docker-compose/model.forge" "Redis"         "docker-compose: detected redis service"
check_contains "$RESULTS/docker-compose/model.forge" "Node.js"       "docker-compose: detected node base image"
check_contains "$RESULTS/docker-compose/model.forge" "depends on"    "docker-compose: detected dependency"

# ── CI / GitHub Actions ──────────────────────────────────────────

run_fixture "ci-github" "ci"
check_contains "$RESULTS/ci-github/model.forge" "pipeline"  "ci-github: detected pipeline"
check_contains "$RESULTS/ci-github/model.forge" "stage"     "ci-github: detected stages"
check_contains "$RESULTS/ci-github/model.forge" "Build"     "ci-github: detected Build job"
check_contains "$RESULTS/ci-github/model.forge" "Test"      "ci-github: detected Test job"
check_contains "$RESULTS/ci-github/model.forge" "Deploy"    "ci-github: detected Deploy job"

# ── Kubernetes ───────────────────────────────────────────────────
# K8s scanner creates DeploymentNode elements which the emitter doesn't
# serialize to .forge (only Person/System/Container/Component are emitted).
# Validate the scanner runs without error and produces a non-empty model.

run_fixture "k8s-manifests" "k8s"
check_file_not_empty "$RESULTS/k8s-manifests/model.forge" "k8s: produced output"

# ── Terraform ────────────────────────────────────────────────────

run_fixture "infra-terraform" "infra"
check_contains "$RESULTS/infra-terraform/model.forge" "Lambda"    "terraform: detected Lambda"
check_contains "$RESULTS/infra-terraform/model.forge" "DynamoDB"  "terraform: detected DynamoDB"
check_contains "$RESULTS/infra-terraform/model.forge" "SQS"       "terraform: detected SQS"

# ── CloudFormation ───────────────────────────────────────────────

run_fixture "infra-cloudformation" "infra"
check_contains "$RESULTS/infra-cloudformation/model.forge" "Lambda"    "cloudformation: detected Lambda"
check_contains "$RESULTS/infra-cloudformation/model.forge" "DynamoDB"  "cloudformation: detected DynamoDB"

# ── OpenAPI ──────────────────────────────────────────────────────
# OpenAPI scanner stores endpoints in api_catalogs (not emitted to .forge)
# but creates a container element for the API title.

run_fixture "openapi-spec" "infra"
check_contains "$RESULTS/openapi-spec/model.forge" "Payment API"    "openapi: detected API title"

# ── Full Stack (all scanners) ────────────────────────────────────

run_fixture "full-stack" "code,semantic,ci,docker,k8s,infra"
check_contains "$RESULTS/full-stack/model.forge" "Rust"         "full-stack: detected Rust project"
check_contains "$RESULTS/full-stack/model.forge" "React"        "full-stack: detected React frontend"
check_contains "$RESULTS/full-stack/model.forge" "pipeline"     "full-stack: detected CI pipeline"
check_contains "$RESULTS/full-stack/model.forge" "PostgreSQL"   "full-stack: detected postgres"
check_count_gt "$RESULTS/full-stack/site" "*.html"              "full-stack: generated HTML pages"

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════"
echo "  Results: $PASSES passed, $FAILURES failed"
echo "════════════════════════════════════════"

if [ "$FAILURES" -eq 0 ]; then
    echo "All integration tests passed."
    exit 0
else
    echo "$FAILURES test(s) FAILED."
    exit 1
fi

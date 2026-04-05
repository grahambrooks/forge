#!/usr/bin/env bash
# Cargo install test for forge.
# Verifies that `cargo install --path forge` works correctly and produces
# a functional binary, matching the documented source installation method.
set -euo pipefail

FIXTURES="${FIXTURES:-/fixtures}"
FAILURES=0
PASSES=0

pass() { PASSES=$((PASSES + 1)); echo "  PASS: $1"; }
fail() { FAILURES=$((FAILURES + 1)); echo "  FAIL: $1"; }

echo ""
echo "=== cargo install test ==="
echo ""

# ── cargo install ────────────────────────────────────────────────

echo "--- cargo install --path ---"

if cargo install --path /build/forge 2>&1; then
    pass "cargo install --path succeeded"
else
    fail "cargo install --path failed"
    exit 1
fi

# Verify cargo installed it to PATH (location varies: ~/.cargo/bin or $CARGO_HOME/bin)
if FORGE=$(which forge 2>/dev/null); then
    pass "forge binary installed at $FORGE"
else
    fail "forge binary not found on PATH"
    exit 1
fi

# Check it's a real ELF binary
if file "$FORGE" | grep -q "ELF"; then
    pass "forge is a native ELF binary"
else
    fail "forge does not appear to be an ELF binary"
fi

# ── Version / help ───────────────────────────────────────────────

echo "--- CLI basics ---"

if VERSION=$($FORGE --version 2>&1); then
    pass "forge --version: $VERSION"
else
    fail "forge --version failed"
fi

if $FORGE --help > /dev/null 2>&1; then
    pass "forge --help exits successfully"
else
    fail "forge --help failed"
fi

# Verify all expected subcommands
HELP_OUTPUT=$($FORGE --help 2>&1)
for cmd in build check analyze generate watch serve lsp export import mcp; do
    if echo "$HELP_OUTPUT" | grep -qi "$cmd"; then
        pass "subcommand '$cmd' listed in help"
    else
        fail "subcommand '$cmd' missing from help"
    fi
done

# ── Functional tests ────────────────────────────────────────────

echo "--- forge build ---"

OUTDIR=$(mktemp -d)
if $FORGE build --source "$FIXTURES/payments.forge" --out "$OUTDIR" 2>&1; then
    pass "forge build completed"
else
    fail "forge build failed"
fi

SVG_COUNT=$(find "$OUTDIR" -name "*.svg" 2>/dev/null | wc -l | tr -d ' ')
if [ "$SVG_COUNT" -gt 0 ]; then
    pass "forge build produced $SVG_COUNT SVG file(s)"
else
    fail "forge build produced no SVG files"
fi
rm -rf "$OUTDIR"

echo "--- forge check ---"

if $FORGE check --source "$FIXTURES/payments.forge" --severity info 2>&1; then
    pass "forge check completed"
else
    fail "forge check crashed"
fi

echo "--- forge export ---"

EXPORT_OUT=$(mktemp)
if $FORGE export --source "$FIXTURES/payments.forge" -f json > "$EXPORT_OUT" 2>&1; then
    pass "forge export completed"
else
    fail "forge export failed"
fi

if [ -s "$EXPORT_OUT" ]; then
    pass "forge export produced non-empty JSON"
else
    fail "forge export produced empty output"
fi
rm -f "$EXPORT_OUT"

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "══════════════════════��═════════════════"
echo "  cargo install test"
echo "  Results: $PASSES passed, $FAILURES failed"
echo "════��════════════════════════��══════════"

if [ "$FAILURES" -eq 0 ]; then
    echo "All cargo install tests passed."
    exit 0
else
    echo "$FAILURES test(s) FAILED."
    exit 1
fi

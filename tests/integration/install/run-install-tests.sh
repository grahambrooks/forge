#!/usr/bin/env bash
# Installation tests for the forge binary.
# Verifies that forge is correctly installed and functional on the current OS.
#
# Expects:
#   - forge binary at /usr/local/bin/forge
#   - test fixture at /fixtures/payments.forge
set -euo pipefail

FORGE="${FORGE:-/usr/local/bin/forge}"
FIXTURES="${FIXTURES:-/fixtures}"
FAILURES=0
PASSES=0

pass() { PASSES=$((PASSES + 1)); echo "  PASS: $1"; }
fail() { FAILURES=$((FAILURES + 1)); echo "  FAIL: $1"; }

DISTRO=$(cat /etc/os-release 2>/dev/null | grep ^PRETTY_NAME | cut -d= -f2 | tr -d '"' || echo "unknown")
echo ""
echo "=== Installation tests on: $DISTRO ==="
echo ""

# ── Binary existence and permissions ─────────────────────────────

echo "--- Binary checks ---"

if [ -x "$FORGE" ]; then
    pass "forge binary is executable"
else
    fail "forge binary missing or not executable at $FORGE"
    echo "Cannot continue without forge binary."
    exit 1
fi

# Check it's a real ELF/binary (not a script or empty file)
if file "$FORGE" | grep -q "ELF"; then
    pass "forge is a native ELF binary"
else
    fail "forge does not appear to be an ELF binary"
fi

# ── Version / help ───────────────────────────────────────────────

echo "--- CLI basics ---"

if $FORGE --help > /dev/null 2>&1; then
    pass "forge --help exits successfully"
else
    fail "forge --help failed"
fi

if VERSION=$($FORGE --version 2>&1); then
    pass "forge --version: $VERSION"
else
    fail "forge --version failed"
fi

# Verify all expected subcommands are present in help output
HELP_OUTPUT=$($FORGE --help 2>&1)
for cmd in build check analyze generate watch serve lsp export import mcp; do
    if echo "$HELP_OUTPUT" | grep -qi "$cmd"; then
        pass "subcommand '$cmd' listed in help"
    else
        fail "subcommand '$cmd' missing from help"
    fi
done

# ── Build command ────────────────────────────────────────────────

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

# ── Check command ────────────────────────────────────────────────

echo "--- forge check ---"

if $FORGE check --source "$FIXTURES/payments.forge" --severity info 2>&1; then
    pass "forge check completed"
else
    # check may exit non-zero for errors, but should not crash
    fail "forge check crashed"
fi

# ── Export command ───────────────────────────────────────────────

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

# ── Generate command ─────────────────────────────────────────────

echo "--- forge generate ---"

SITE_DIR=$(mktemp -d)
if $FORGE generate --source "$FIXTURES/payments.forge" --out "$SITE_DIR" 2>&1; then
    pass "forge generate completed"
else
    fail "forge generate failed"
fi

HTML_COUNT=$(find "$SITE_DIR" -name "*.html" 2>/dev/null | wc -l | tr -d ' ')
if [ "$HTML_COUNT" -gt 0 ]; then
    pass "forge generate produced $HTML_COUNT HTML file(s)"
else
    fail "forge generate produced no HTML files"
fi
rm -rf "$SITE_DIR"

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════"
echo "  $DISTRO"
echo "  Results: $PASSES passed, $FAILURES failed"
echo "════════════════════════════════════════"

if [ "$FAILURES" -eq 0 ]; then
    echo "All installation tests passed."
    exit 0
else
    echo "$FAILURES test(s) FAILED."
    exit 1
fi

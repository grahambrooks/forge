#!/usr/bin/env bash
# Homebrew installation test for forge.
# Sets up a local Homebrew tap with the forge formula, installs it using
# a locally-built binary, and verifies the installed version works correctly.
set -euo pipefail

FIXTURES="${FIXTURES:-/fixtures}"
FAILURES=0
PASSES=0

pass() { PASSES=$((PASSES + 1)); echo "  PASS: $1"; }
fail() { FAILURES=$((FAILURES + 1)); echo "  FAIL: $1"; }

echo ""
echo "=== Homebrew installation test ==="
echo ""

# ── Verify Homebrew is available ─────────────────────────────────

echo "--- Homebrew setup ---"

if brew --version > /dev/null 2>&1; then
    pass "brew is installed: $(brew --version | head -1)"
else
    fail "brew not found"
    exit 1
fi

# ── Create a local tap with the formula ──────────────────────────
# We rewrite the formula to point at a local file:// URL instead of
# a GitHub release, so we can test the real Homebrew install flow
# without needing a published release.

echo "--- Setting up local tap ---"

TAP_DIR="$(brew --repository)/Library/Taps/local/homebrew-forge"
mkdir -p "$TAP_DIR/Formula"

# Initialize as a git repo (Homebrew requires taps to be git repos)
cd "$TAP_DIR"
git init -q
git config user.email "test@test.com"
git config user.name "Test"

# Detect version from the binary itself so the formula always matches
EXPECTED_VERSION=$(/tmp/forge-binary --version 2>&1 | awk '{print $2}')
BINARY_SHA=$(sha256sum /tmp/forge-binary | awk '{print $1}')

echo "  Binary reports version: $EXPECTED_VERSION"

cat > "$TAP_DIR/Formula/forge.rb" << FORMULA
class Forge < Formula
  desc "Unified software modeling DSL"
  homepage "https://github.com/grahambrookss/forge"
  version "${EXPECTED_VERSION}"
  license "MIT"

  url "file:///tmp/forge-binary"
  sha256 "${BINARY_SHA}"

  def install
    bin.install "forge-binary" => "forge"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/forge --version")
  end
end
FORMULA

git add -A
git commit -q -m "Add forge formula"

pass "local tap created at $TAP_DIR"

# ── Install via brew ────────��────────────────────────────────────

echo "--- brew install ---"

if brew install local/forge/forge 2>&1; then
    pass "brew install succeeded"
else
    fail "brew install failed"
    # Show diagnostics
    brew config 2>&1 || true
    exit 1
fi

# Verify brew knows about the package
if brew list local/forge/forge > /dev/null 2>&1; then
    pass "forge appears in brew list"
else
    fail "forge not in brew list"
fi

# ── Verify the installed binary ──────────────────────────────────

echo "--- Installed binary checks ---"

FORGE="$(brew --prefix)/bin/forge"

if [ -x "$FORGE" ]; then
    pass "forge binary is executable at $FORGE"
else
    fail "forge binary missing or not executable at $FORGE"
    exit 1
fi

if VERSION=$($FORGE --version 2>&1); then
    pass "forge --version: $VERSION"
else
    fail "forge --version failed"
fi

# Verify installed version matches what brew expects
INSTALLED_VERSION=$($FORGE --version 2>&1 | awk '{print $2}')
BREW_VERSION=$(brew list --versions local/forge/forge 2>/dev/null | awk '{print $2}')
if [ -n "$BREW_VERSION" ] && [ "$INSTALLED_VERSION" = "$BREW_VERSION" ]; then
    pass "binary version ($INSTALLED_VERSION) matches brew version ($BREW_VERSION)"
else
    fail "version mismatch: binary=$INSTALLED_VERSION brew=$BREW_VERSION"
fi

if $FORGE --help > /dev/null 2>&1; then
    pass "forge --help exits successfully"
else
    fail "forge --help failed"
fi

# ── Functional tests ────────���────────────────────────────────────

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

# ── brew test ────────────────────────────────────────────────────

echo "--- brew test ---"

if brew test local/forge/forge 2>&1; then
    pass "brew test passed"
else
    fail "brew test failed"
fi

# ── brew uninstall ────���─────────────────────────────────��────────

echo "--- brew uninstall ---"

if brew uninstall local/forge/forge 2>&1; then
    pass "brew uninstall succeeded"
else
    fail "brew uninstall failed"
fi

if ! command -v forge > /dev/null 2>&1; then
    pass "forge no longer on PATH after uninstall"
else
    fail "forge still on PATH after uninstall"
fi

# ── Summary ──────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════"
echo "  Homebrew installation test"
echo "  Results: $PASSES passed, $FAILURES failed"
echo "════════════════════════════════════════"

if [ "$FAILURES" -eq 0 ]; then
    echo "All Homebrew installation tests passed."
    exit 0
else
    echo "$FAILURES test(s) FAILED."
    exit 1
fi

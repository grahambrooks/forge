# Install

Forge ships as a single binary called `forge`. You need it on your `$PATH`
before you can run any of the other commands in this guide.

## Homebrew (recommended)

```bash
brew tap grahambrooks/forge https://github.com/grahambrooks/forge
brew install grahambrooks/forge/forge
```

The formula lives in the Forge repo itself rather than a separate
`homebrew-forge` tap, so the tap URL is required. The repo is public — no
GitHub token or `HOMEBREW_GITHUB_API_TOKEN` is needed.

Covers macOS on Apple Silicon and Linux on x86_64 and aarch64. Intel Macs
have no prebuilt binary; use [From source](#from-source).

## Prebuilt binary

Each release publishes tarballs on
[GitHub Releases](https://github.com/grahambrooks/forge/releases):

```bash
TAG=$(curl -sL https://api.github.com/repos/grahambrooks/forge/releases/latest \
      | grep -m1 '"tag_name"' | cut -d'"' -f4)

# One of: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin
TARGET=aarch64-apple-darwin

curl -L "https://github.com/grahambrooks/forge/releases/download/${TAG}/forge-${TAG}-${TARGET}.tar.gz" | tar xz
chmod +x forge && mv forge ~/.local/bin/forge
```

## With cargo

The binary crate is published as `forge-dsl`. This installs a `forge`
executable under `~/.cargo/bin`:

```bash
cargo install --locked forge-dsl
forge --help
```

Prerequisites: a stable Rust toolchain (1.80 or newer) and a C toolchain for
building the tree-sitter grammars that `symgraph` pulls in (Xcode Command
Line Tools on macOS; `build-essential` on Debian/Ubuntu).

To install unreleased changes from `main`, point cargo at the repo instead.
The `forge-dsl` package name is required because the repo also carries example
crates:

```bash
cargo install --git https://github.com/grahambrooks/forge forge-dsl --locked
```

## From source

Clone the repo and build in release mode. This is the way to work on Forge
itself, or to try changes on `main` that are not in a release yet.

```bash
git clone https://github.com/grahambrooks/forge.git
cd forge/forge
cargo build --release
./target/release/forge --help
```

Copy the binary into your path if you want to use it everywhere:

```bash
cp target/release/forge ~/.local/bin/forge
```

## Verify the install

```bash
forge --version
forge --help
```

You should see the top-level subcommand list:

```
Usage: forge <COMMAND>

Commands:
  build     Parse .forge files and render SVG diagrams
  check     Lint and validate a model against architectural rules
  analyze   Scan codebases and produce a .forge model
  generate  Generate a static documentation website
  export    Export model as JSON or YAML
  import    Import from PlantUML C4 or Mermaid to .forge
  watch     Watch for changes and rebuild automatically
  serve     Start a local preview server with live reload
  mcp       Start the MCP server for AI agent integration
  lsp       Start the Language Server Protocol server
```

## Editor setup

Forge ships a Language Server Protocol implementation (`forge lsp`) that
provides diagnostics, hover, completion, and go-to-definition inside any
editor that speaks LSP.

See [`forge/EDITORS.md`](../../forge/EDITORS.md) for copy-paste configs for
VS Code, Neovim, Helix, Emacs, IntelliJ, Sublime Text, and Zed.

## Next

- [Your first model](first-model.md) if you want to hand-write a `.forge` file.
- [Analyzing a codebase](analyzing-a-codebase.md) if you want `forge` to build
  one automatically from an existing repo.

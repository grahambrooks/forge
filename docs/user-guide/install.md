# Install

Forge ships as a single binary called `forge`. You need it on your `$PATH`
before you can run any of the other commands in this guide.

## From crates.io (recommended)

The binary crate is published as `forge-dsl`. This installs a `forge`
executable under `~/.cargo/bin`:

```bash
cargo install --locked forge-dsl
forge --help
```

Prerequisites: a stable Rust toolchain (1.80 or newer) and a C toolchain for
building the tree-sitter grammars that `symgraph` pulls in (Xcode Command
Line Tools on macOS; `build-essential` on Debian/Ubuntu).

## From source

Clone the repo and build in release mode. This is the fastest way to try
unreleased features that are on `main` but not yet published to crates.io.

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

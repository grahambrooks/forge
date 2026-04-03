# Forge — Claude Code Project Guide

## What is Forge?

Forge is a unified software modeling DSL and toolchain that describes both the *structure* (C4 architecture) and *processes* (CI/CD pipelines, branching strategies) of software systems from a single coherent model. It renders multiple views as clean SVG with CSS classes, generates static documentation sites, lints architecture, and exposes capabilities via MCP.

Read `DESIGN.md` for the full specification. It is the source of truth for all design decisions.

## Project Structure

```
model-diagram/
├── DESIGN.md                    # Complete design specification (read this first)
├── CLAUDE.md                    # This file
└── forge/
    ├── examples/
    │   └── payments.forge       # Reference DSL example (payments platform)
    ├── src/                     # Rust implementation
    │   ├── main.rs              # CLI entry point (clap): build, check, analyze subcommands
    │   ├── model.rs             # Semantic model (ElementKind, Element, Relationship, View, Model)
    │   ├── parser.rs            # Hand-written recursive descent parser
    │   ├── check.rs             # Architectural linter (8 built-in rules)
    │   ├── analyze/             # Codebase scanners
    │   │   ├── mod.rs           # Scanner orchestration and config
    │   │   ├── code.rs          # Project manifest scanner (Cargo.toml, package.json, go.mod, etc.)
    │   │   ├── ci.rs            # CI/CD scanner (GitHub Actions YAML)
    │   │   ├── docker.rs        # Docker scanner (Dockerfile, docker-compose.yml)
    │   │   └── emit.rs          # Model → .forge DSL emitter
    │   ├── generate.rs          # Static documentation site generator
    │   ├── layout.rs            # Layout algorithms (system context, container, pipeline)
    │   └── render.rs            # SVG renderer (filled + outline modes, C4 palette)
    ├── output/                  # Generated SVGs and PNGs (both filled + outline)
    │   └── preview.html         # HTML comparison page with style toggle
    └── Cargo.toml               # Rust project config (clap dependency)
```

## Current State

### What works (Rust implementation)
- Full hand-written recursive descent parser for the Forge DSL
- Semantic model with typed elements, relationships, and views
- Three layout algorithms: system context, container (layered), pipeline (topological sort)
- SVG renderer with two modes: **filled** (canonical C4 colors) and **outline** (wireframe)
- Structurizr-style rendering: person silhouette, database cylinder, gate diamond, drop shadows, legend, edge label pills
- Complex shapes render cleanly in outline mode (single unified paths, no internal construction lines)
- CLI with `build`, `check`, `analyze`, and `generate` subcommands (via `clap`)
- Working example: `payments.forge` producing SystemContext, Containers, and Pipeline views
- Output matches the Python prototype (16 elements, 5 relationships, 3 views)
- Architectural linter with 8 built-in rules: dependency-cycles, orphaned-elements, missing-descriptions, missing-technology, database-direct-access, chatty-coupling, gate-coverage, empty-views
- Lint output in text or JSON format, configurable severity filter, exit codes for CI integration
- Codebase analyzer with 3 scanners: code (Cargo.toml, package.json, go.mod, pyproject.toml, pom.xml, build.gradle), CI (GitHub Actions), Docker (Dockerfile, docker-compose.yml)
- .forge DSL emitter with round-trip support (parse → emit → re-parse)
- Static documentation site generator: index page with model overview and check results, per-view pages with embedded SVGs, per-element detail pages with relationships, JSON export, responsive CSS theme with sidebar navigation

### What needs to be built
The remaining commands from DESIGN.md (`generate`, `mcp`, `watch`, `serve`, `export`, `import`, `lsp`), additional analyze scanners (git, k8s, openapi, tree-sitter code analysis), custom lint rules, and the Cargo workspace restructuring.

## Key Commands

### Build and run
```bash
cd forge
cargo build
cargo run -- build --source examples/payments.forge --out output
cargo run -- build --source examples/payments.forge --out output --style outline
cargo run -- check --source examples/payments.forge
cargo run -- check --source examples/payments.forge --severity info --format json
cargo run -- analyze ./path/to/project --out project.forge
cargo run -- analyze --dry-run --scanners code,ci .
cargo run -- generate --source examples/payments.forge --out _site
```

### View results
Open `forge/output/preview.html` in a browser for side-by-side filled/outline comparison.

## Architecture Decisions

1. **Single Rust binary** — Everything compiles into one statically-linked binary: parser, renderer, analyzer, checker, site generator, MCP server. Feature flags for optional deps (resvg, tower-lsp, MCP, tantivy).

2. **Cargo workspace layout** — Use a Cargo workspace with internal crates (`forge-parser`, `forge-model`, `forge-analyze`, `forge-check`, `forge-layout`, `forge-render`, `forge-sitegen`, `forge-diff`, `forge-mcp`, `forge-cli`, `forge-lsp`) that all compile into a single `[[bin]]` target.

3. **Key Rust dependencies** — `pest` (parser), `tree-sitter` (code analysis), `gix` (git), `clap` (CLI), `serde`/`serde_json`, `tera` (HTML templates), `tokio` (async for MCP/LSP/serve). Optional: `resvg`, `tower-lsp`, `rmcp`, `tantivy`, `gifski`.

4. **SVG rendering** — Hand-written SVG with semantic CSS classes (`.forge-element--container`, `.forge-diff--added`, etc.). No DOM library. Two rendering modes: filled and outline.

5. **MCP server** — Exposes tools: `forge_query`, `forge_render`, `forge_check`, `forge_diff`, `forge_analyze`, `forge_element_detail`, `forge_search`, `forge_validate`, `forge_suggest_fix`. Stdio and HTTP transports.

## CLI Commands (target)

| Command | Status | Description |
|---------|--------|-------------|
| `forge build` | Working | Parse .forge → render SVGs |
| `forge analyze` | Working (code, ci, docker scanners) | Scan codebases → generate .forge model |
| `forge generate` | Working | Model → static documentation website |
| `forge generate --diff` | Design complete | Highlight architectural changes |
| `forge check` | Working | Lint model against architectural rules |
| `forge mcp` | Design complete | MCP server for AI agent access |
| `forge watch` | Design complete | Incremental rebuild on file changes |
| `forge serve` | Design complete | Local preview server with live reload |
| `forge export` | Design complete | Export as JSON/YAML |
| `forge import` | Design complete | Import from PlantUML/Mermaid |
| `forge lsp` | Working | Language Server Protocol (see EDITORS.md) |

## DSL Quick Reference

```forge
forge "System Name" {
  model {
    actor = person "Name" { description "..." }
    sys = system "Name" {
      svc = container "Name" { technology "Rust / Axum" }
      db  = container "DB" { technology "PostgreSQL"; tags "database" }
      svc -> db "reads/writes" "SQL"
    }
    actor -> sys.svc "uses" "HTTPS"
  }

  process {
    pipeline "ci" {
      build = stage "Build" { step "cargo build" }
      deploy = stage "Deploy" { needs build; gate "tests-pass" }
    }
  }

  views {
    systemContext sys "Context" { include *; autoLayout lr }
    container sys "Containers" { include *; autoLayout tb }
    pipelineView "ci" "Pipeline" { include *; autoLayout lr }
  }
}
```

## Coding Conventions

- **Rust style**: Use `rustfmt` defaults. Error handling via `thiserror` for library crates, `anyhow` for CLI.
- **SVG output**: All elements get semantic CSS classes. Use `forge-` prefix for all class names. Keep SVG self-contained (inline styles, no external dependencies).
- **Testing**: Each crate should have unit tests. Integration tests parse `.forge` files from `examples/` and verify SVG output structure.
- **File naming**: Crate names use hyphens (`forge-parser`), module files use underscores per Rust convention.

## Next Steps (Priority Order)

1. **Restructure into Cargo workspace** with internal crates matching DESIGN.md §4.2 (`forge-parser`, `forge-model`, `forge-layout`, `forge-render`, `forge-check`, `forge-cli`)
2. **Add custom rule support** — `.forge-rules` declarative syntax from DESIGN.md §8.5
4. **Implement `forge analyze`** starting with the code scanner (tree-sitter) and git scanner (gix)
5. **Implement `forge generate`** for static site output
6. **Implement `forge diff`** for model comparison
7. **Implement `forge mcp`** server

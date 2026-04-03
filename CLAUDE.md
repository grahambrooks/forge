# Forge — Claude Code Project Guide

## What is Forge?

Forge is a unified software modeling DSL and toolchain that describes both the *structure* (C4 architecture) and *processes* (CI/CD pipelines, branching strategies) of software systems from a single coherent model. It renders multiple views as clean SVG with CSS classes, generates static documentation sites, lints architecture, and provides IDE integration via LSP.

Read `DESIGN.md` for the full specification. It is the source of truth for all design decisions.

## Project Structure

```
model-diagram/
├── DESIGN.md                    # Complete design specification
├── CLAUDE.md                    # This file
└── forge/
    ├── examples/
    │   ├── payments.forge       # Full reference example (30 elements, 11 views)
    │   ├── payments-baseline.forge  # Baseline for diff demo
    │   ├── team-rules.forge-rules   # Custom lint rules example
    │   ├── docs/                # Markdown documentation (5 pages + ADR)
    │   └── multi-file/          # Multi-file !include example
    ├── src/
    │   ├── main.rs              # CLI (12 subcommands via clap)
    │   ├── model.rs             # Semantic model (elements, relationships, views, animation, data model, teams, trust boundaries, tech stack)
    │   ├── parser.rs            # Hand-written recursive descent parser
    │   ├── preprocess.rs        # !include, !fragment/!use, !if directives
    │   ├── check.rs             # 8 built-in architectural lint rules
    │   ├── custom_rules.rs      # Declarative .forge-rules engine
    │   ├── diff.rs              # Model differencing engine
    │   ├── animate.rs           # Frame-based SVG animation with CSS keyframes
    │   ├── analyze/             # Codebase scanners (code, ci, docker) + .forge emitter
    │   ├── generate.rs          # Static site generator (HTML, CSS, JSON)
    │   ├── layout.rs            # Content-aware layout algorithms (9 view types)
    │   ├── render.rs            # SVG renderer (filled + outline, entity tables, branches)
    │   ├── lsp.rs               # Language Server Protocol (tower-lsp)
    │   └── serve.rs             # File watcher + live-reload HTTP server
    ├── EDITORS.md               # LSP setup for 7 editors
    ├── PUBLISHING.md            # GitHub Pages + Backstage TechDocs deployment
    ├── Cargo.toml               # Dependencies: clap, serde, tokio, tower-lsp, notify, pulldown-cmark
    └── Makefile                 # build, test, lint, fmt, pre-commit, run
```

## Current State (139 tests, all passing)

### CLI Commands

| Command | Status | Description |
|---------|--------|-------------|
| `forge build` | Working | Parse .forge → render SVGs (11 view types) |
| `forge check` | Working | 8 built-in rules + custom `.forge-rules` |
| `forge analyze` | Working | Scan codebases (code, CI, Docker, git) → .forge |
| `forge generate` | Working | Model → static documentation website |
| `forge generate --baseline` | Working | Diff highlighting (green/amber) vs baseline |
| `forge watch` | Working | Auto-rebuild on .forge/.md changes |
| `forge serve` | Working | Live-reload preview server (SSE) + `--present` mode |
| `forge lsp` | Working | IDE integration (diagnostics, hover, completion, go-to-def) |
| `forge mcp` | Working | MCP server for AI agent access (6 tools) |
| `forge export` | Working | JSON/YAML model export |
| `forge import` | Working | Import from PlantUML C4 / Mermaid flowchart |
| `forge check --format sarif` | Working | SARIF 2.1.0 output for GitHub Code Scanning |

### View Types (11)

| View | DSL keyword | Description |
|------|------------|-------------|
| System Context | `systemContext` | Actors and systems |
| Container | `container` | Containers within a system |
| Component | `component` | Components within a container |
| Pipeline | `pipelineView` | CI/CD stages and gates |
| Deployment | `deploymentView` | Infrastructure topology with nested nodes |
| Tech Stack | `techStackView` | Technology inventory by category |
| Branching | `branchingView` | Git branching strategy |
| Data Model | `dataModelView` | Entity-relationship diagram with field tables |
| Trust Boundaries | `trustBoundaryView` | Security zones with members |
| Team Ownership | `teamView` | Team → container ownership map |
| Animated | Any view + `animation {}` | Frame-based step-by-step walkthrough |

### DSL Features

- **Structural**: person, system, container, component, relationships
- **Process**: repository, strategy/branch, pipeline/stage/gate
- **Deployment**: nested deployment nodes with container instances
- **Data**: entity/field/relationship with types and constraints
- **Security**: trust boundary zones (public, dmz, internal, pci)
- **Teams**: team definitions with ownership and contact info
- **Tech Stack**: categorized technology inventory with versions
- **Documentation**: markdown docs and ADRs
- **Animation**: frame-based reveal with highlights, pulse, state changes
- **Preprocessor**: `!include` (path + glob), `!fragment`/`!use`, `!if env()`
- **Custom rules**: `.forge-rules` declarative lint syntax

## Key Commands

```bash
cd forge
cargo build

# Build SVGs
cargo run -- build --source examples/payments.forge --out output
cargo run -- build --source examples/payments.forge --out output --style outline

# Check architecture
cargo run -- check --source examples/payments.forge
cargo run -- check --source examples/payments.forge --rules examples/team-rules.forge-rules

# Analyze a codebase
cargo run -- analyze ./path/to/project --out project.forge
cargo run -- analyze --dry-run --scanners code,ci .

# Generate documentation site
cargo run -- generate --source examples/payments.forge --out _site
cargo run -- generate --source examples/payments.forge --baseline examples/payments-baseline.forge --out _site

# Live development
cargo run -- serve --source examples/payments.forge --port 4000
cargo run -- watch --source examples/payments.forge

# LSP server (for editors)
cargo run -- lsp
```

## Coding Conventions

- **Rust style**: `rustfmt` defaults. Run `make pre-commit` before committing (fmt + clippy + test).
- **SVG output**: All elements get semantic CSS classes with `forge-` prefix. SVGs are self-contained.
- **Testing**: Unit tests in each module. Integration tests parse `examples/payments.forge`.
- **Raw strings**: Use `r##"..."##` for strings containing `"#` (common in SVG color values).

## What Needs to Be Built

### High Priority
1. **`forge mcp`** — MCP server exposing all capabilities to AI agents (Claude Code, Cursor, Windsurf)
2. **`forge export`** — Standalone JSON/YAML export command (JSON already exists inside generate)
3. **More analyze scanners** — git (branching/ownership via gix), k8s manifests, OpenAPI specs

### Medium Priority
4. **`forge import`** — Import from PlantUML/Mermaid formats
5. **SARIF output** — `forge check --format sarif` for GitHub Code Scanning
6. **PNG/PDF export** — Via `resvg`
7. **Client-side search** — Search index for generated sites

### Lower Priority
8. **Cargo workspace restructure** — Split into forge-parser, forge-model, etc. crates
9. **Tree-sitter code analysis** — Full AST parsing for import graphs
10. **Force-directed layout** — For landscape/overview diagrams
11. **Cross-compilation** — Release binaries for Linux, macOS, Windows

### Phase 6 (Future — documented in DESIGN.md)
- API catalog, event/message flows, environment config, SLA/SLO definitions, data classification, on-call/runbook links

# Forge

A unified software modeling DSL and toolchain. Describe your architecture — structure, processes, deployment, and data — in a single `.forge` file. Render diagrams, generate documentation sites, lint architecture, and integrate with AI agents.

## Quick Start

```bash
# Install (see Install below for Homebrew and prebuilt binaries)
cargo install --locked forge-dsl

# Create a model
cat > architecture.forge << 'EOF'
forge "My Platform" {
  model {
    user = person "User" { description "End user" }
    api = system "API" {
      web = container "Web API" { technology "Node.js / Express" }
      db  = container "Database" { technology "PostgreSQL"; tags "database" }
      web -> db "reads/writes" "SQL"
    }
    user -> api.web "uses" "HTTPS"
  }
  views {
    systemContext api "Context" { include *; autoLayout lr }
    container api "Containers" { include *; autoLayout tb }
  }
}
EOF

# Build SVG diagrams
forge build -s architecture.forge

# Generate a documentation site
forge generate -s architecture.forge -o _site

# Generate a multi-project catalog (enterprise scale)
forge generate-catalog -s enterprise.catalog -o _site

# Start a live-reload preview
forge serve -s architecture.forge

# Check for architectural issues
forge check -s architecture.forge

# Scan a codebase to generate a model
forge analyze ./my-project -o discovered.forge
```

## Features

- **11 view types**: System context, container, component, deployment, pipeline, branching, data model, trust boundaries, team ownership, tech stack, animated
- **Multi-project catalogs**: Aggregate 1000s of models into one site with incremental builds ([CATALOG.md](./CATALOG.md))
- **Living documentation**: `forge analyze` scans code → generates models → `forge generate` publishes sites
- **Diff highlighting**: Compare models and highlight changes in green/amber
- **Architecture linting**: 8 built-in rules + custom `.forge-rules` files with SARIF output
- **LSP integration**: VS Code, Neovim, Emacs, Zed — autocomplete, diagnostics, hover ([EDITORS.md](./forge/EDITORS.md))
- **MCP server**: AI agent access via Model Context Protocol (`forge mcp`)
- **Single binary**: No JVM, no Node, no Python — one statically-linked Rust binary

## Install

### Homebrew

```bash
brew tap grahambrooks/forge https://github.com/grahambrooks/forge
brew install grahambrooks/forge/forge
```

The formula lives in this repo rather than a separate `homebrew-forge` tap, so
the tap URL is required. No GitHub token is needed.

### Download a binary

Each release publishes tarballs on
[GitHub Releases](https://github.com/grahambrooks/forge/releases) for Linux
(x86_64 and aarch64) and macOS (Apple Silicon):

```bash
TAG=$(curl -sL https://api.github.com/repos/grahambrooks/forge/releases/latest \
      | grep -m1 '"tag_name"' | cut -d'"' -f4)

# One of: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin
TARGET=aarch64-apple-darwin

curl -L "https://github.com/grahambrooks/forge/releases/download/${TAG}/forge-${TAG}-${TARGET}.tar.gz" | tar xz
chmod +x forge && sudo mv forge /usr/local/bin/
```

Intel Macs have no prebuilt binary — build from source instead.

### With cargo

The crate is published as `forge-dsl` and installs a `forge` binary:

```bash
cargo install --locked forge-dsl
```

### Build from source

To install unreleased changes from `main`, no clone required. The package name
is needed because the repo also carries example crates:

```bash
cargo install --git https://github.com/grahambrooks/forge forge-dsl --locked
```

Or clone first if you want to work on Forge itself:

```bash
git clone https://github.com/grahambrooks/forge.git
cd forge/forge
cargo install --locked --path .
```

## Commands

| Command | Description |
|---------|-------------|
| `forge build` | Parse `.forge` files and render SVG diagrams |
| `forge check` | Lint architecture (8 built-in rules + custom `.forge-rules`) |
| `forge analyze` | Scan codebases and produce a `.forge` model |
| `forge generate` | Generate a static documentation website |
| `forge export` | Export model as JSON or YAML |
| `forge import` | Import from PlantUML C4 or Mermaid |
| `forge watch` | Auto-rebuild on file changes |
| `forge serve` | Live-reload preview server with presentation mode |
| `forge mcp` | MCP server for AI agents (Claude Code, Cursor) |
| `forge lsp` | Language Server Protocol for IDE integration |

## What You Can Model

Forge produces **13 view types** from a single DSL:

| View | What it shows |
|------|--------------|
| System Context | Actors and systems |
| Container | Services within a system |
| Component | Internals of a container |
| Pipeline | CI/CD stages and quality gates |
| Deployment | Infrastructure topology (nested nodes) |
| Tech Stack | Technology inventory by category |
| Branching | Git branching strategy |
| Data Model | Entity-relationship diagram |
| Trust Boundaries | Security zones |
| Team Ownership | Who owns what |
| API Catalog | Endpoints grouped by service |
| Event Flows | Publisher / topic / subscriber |
| Animated | Step-by-step walkthrough of any view |

Plus: markdown documentation, architectural diff highlighting, dark/light mode, and presentation mode for walkthroughs.

## DSL Overview

```forge
forge "Payment Platform" {
  model {
    customer = person "Customer" { description "End user making payments" }
    payments = system "Payment Service" {
      api = container "Payment API" { technology "Rust / Actix" }
      db  = container "Ledger DB" { technology "PostgreSQL 16"; tags "database" }
      api -> db "reads/writes" "SQL"
    }
    customer -> payments.api "makes payments" "HTTPS"
  }

  process {
    pipeline "ci" {
      build = stage "Build" { step "cargo build" }
      test = stage "Test" { needs build; gate "tests-pass" }
    }
  }

  deployment "production" {
    node "AWS" {
      node "EKS Cluster" { technology "Kubernetes"; instance api }
      node "RDS" { technology "Managed PostgreSQL"; instance db }
    }
  }

  techStack {
    category "Backend" {
      tech "Rust" { version "1.75"; purpose "API and processing" }
      tech "PostgreSQL" { version "16"; purpose "Transaction ledger" }
    }
  }

  views {
    systemContext payments "Context" { include *; autoLayout lr }
    container payments "Containers" { include *; autoLayout tb }
    pipelineView "ci" "Pipeline" { include *; autoLayout lr }
    deploymentView "production" "Deploy" { include *; autoLayout tb }
    techStackView "Stack" { include *; title "Tech Stack" }
  }

  docs {
    doc "Overview" "docs/overview.md"
    doc "ADR-001" "docs/adr-001.md"
  }
}
```

## Analyze Real Codebases

`forge analyze` scans source code, infrastructure, and CI configs with 7 scanners:

**Languages:** TypeScript/JavaScript (Express, NestJS), Java (Spring Boot, JAX-RS), Go (Gin, Echo), Rust (Actix, Axum), Python (Flask, FastAPI, Django)

**Infrastructure:** Kubernetes manifests, CloudFormation, Terraform (AWS/GCP/Azure), OpenAPI/Swagger, Dockerfiles, docker-compose

**CI/CD:** GitHub Actions workflows

**Git:** Branching strategy detection, contributor analysis

```bash
forge analyze ./my-project -o architecture.forge
forge analyze ./svc-a ./svc-b --scanners code,source,k8s,infra
```

## AI Agent Integration (MCP)

Forge exposes an MCP server for AI agents:

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge",
      "args": ["mcp", "--source", "architecture.forge"]
    }
  }
}
```

**Tools:** `forge_query`, `forge_render`, `forge_check`, `forge_element_detail`, `forge_search`, `forge_validate`

Works with Claude Code, Cursor, Windsurf, and any MCP-compatible client.

## Integrations

### MkDocs

Use the Forge MkDocs plugin to embed diagrams in your documentation:

```yaml
# mkdocs.yml
plugins:
  - forge:
      source: architecture.forge
      style: filled
```

Then reference views in markdown:

```markdown
## System Architecture

{{ "{{" }} forge_view "SystemContext" {{ "}}" }}

## Container Details

{{ "{{" }} forge_view "Containers" {{ "}}" }}
```

Or use the CLI to pre-build SVGs:

```bash
forge build -s architecture.forge -o docs/diagrams/
```

See the [MkDocs integration guide](integrations/mkdocs/) for the full plugin.

### Backstage TechDocs

See the [Backstage integration example](integrations/backstage/) for a complete setup with `catalog-info.yaml`, CI workflow, and TechDocs publishing.

### GitHub Actions

```yaml
- name: Check architecture
  run: forge check -s architecture.forge --format sarif > results.sarif

- name: Generate site
  run: forge generate -s architecture.forge -o _site --baseline baseline.forge
```

### IDE Support

The LSP server provides diagnostics, hover, completion, go-to-definition, and document symbols.

See **[EDITORS.md](forge/EDITORS.md)** for setup: VS Code, Neovim, Helix, Zed, Sublime Text, JetBrains, Emacs

### Publishing

See **[PUBLISHING.md](forge/PUBLISHING.md)** for deployment: GitHub Pages, Backstage TechDocs, Netlify, Vercel, AWS S3, Docker

## File Composition

Split large models across files:

```forge
forge "Platform" {
  !include model/people.forge
  !include model/*.forge
  !include process/pipelines.forge

  !if env("FORGE_ENV") == "production" {
    !include model/prod-only.forge
  }
}
```

## Custom Lint Rules

```forge-rules
rule "max-container-coupling" {
    severity error
    scope container
    condition count(outgoing_relationships) > 5
    message "{element.name} has {count} outgoing deps (max 5)"
}
```

```bash
forge check -s arch.forge --rules team-rules.forge-rules
forge check -s arch.forge -f sarif > results.sarif
```

## Contributing

See **[CONTRIBUTING.md](./CONTRIBUTING.md)** for setup, the `make pre-commit`
gate, testing conventions, and the release process. [DESIGN.md](./DESIGN.md) is
the source of truth for design decisions.

## License

MIT

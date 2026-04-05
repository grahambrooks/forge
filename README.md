# Forge

A unified software modeling DSL and toolchain. Describe your architecture — structure, processes, deployment, and data — in a single `.forge` file. Render diagrams, generate documentation sites, lint architecture, and integrate with AI agents.

## Quick Start

```bash
# Install
cargo install --path forge

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

# Start a live-reload preview
forge serve -s architecture.forge

# Check for architectural issues
forge check -s architecture.forge

# Scan a codebase to generate a model
forge analyze ./my-project -o discovered.forge
```

## Install

### Homebrew

```bash
brew tap grahambrooks/forge https://github.com/grahambrooks/forge
brew install grahambrooks/forge/forge
```

### Download a binary

Download the latest release from [GitHub Releases](https://github.com/grahambrooks/forge/releases):

```bash
# Linux (x86_64)
curl -L https://github.com/grahambrooks/forge/releases/latest/download/forge-linux-x86_64 -o forge
chmod +x forge && sudo mv forge /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/grahambrooks/forge/releases/latest/download/forge-macos-aarch64 -o forge
chmod +x forge && sudo mv forge /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/grahambrooks/forge/releases/latest/download/forge-macos-x86_64 -o forge
chmod +x forge && sudo mv forge /usr/local/bin/
```

### Build from source

```bash
git clone https://github.com/grahambrooks/forge.git
cd forge/forge
cargo install --path .
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

## License

MIT

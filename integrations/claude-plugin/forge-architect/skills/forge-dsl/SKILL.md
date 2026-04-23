---
name: forge-dsl
description: Author or edit Forge `.forge` DSL files — the source of truth for a Forge architecture model. Use when the user asks to "write a .forge file", "add a container to the model", "fix a syntax error", "rename an element", or any direct editing of `.forge` / `.forge-rules` content. Pairs with model-repository (to generate) and architecture-review (to query).
---

# Forge DSL Authoring

The Forge DSL describes a software system's structure (C4 containers/components), delivery process (pipelines, branching), deployment topology, data model, trust boundaries, and teams — in one file. Generated diagrams, docs sites, and lint results all derive from it.

When editing, validate before saving: the MCP server's `forge_validate` tool parses a snippet and returns line/column diagnostics. The LSP also works, if the user has it configured.

## File skeleton

```forge
forge "Model Name" {
  description "Optional one-liner."

  model {
    // Structural elements and their relationships go here.
  }

  process {
    // Repositories, branching strategies, pipelines.
  }

  deployment <environment-id> "Label" {
    // Nested deployment nodes and container instances.
  }

  tech-stack { /* categorised tech inventory */ }
  data-model { /* entities, fields, relations */ }
  trust-boundaries { /* security zones */ }
  teams { /* ownership */ }

  views { /* view declarations */ }
  docs  { /* markdown doc refs */ }
}
```

All keywords are **kebab-case** (`tech-stack`, `data-model`, `container-view`). Strings use double quotes. Identifiers on the left of `=` are ids; the DSL is case-sensitive.

## Structural elements

```forge
// Person — an external user/actor
customer = person "Customer" {
  description "End user making payments"
}

// System — a logical product; contains containers
payments = system "Payment Platform" {
  description "Processes card and bank payments"
  tags "core" "pci"

  // Container — a deployable unit (service, database, SPA, mobile app)
  api = container "Payment API" {
    technology "Rust / Actix"
    description "REST + gRPC gateway"

    // Component — an in-process module
    rest = component "REST Controller" {
      technology "Actix-web"
    }
  }

  db = container "Ledger DB" {
    technology "PostgreSQL 16"
    tags "database"
    data-class "pii" "financial"
  }

  // Relationship — arrow with label and optional technology
  api -> db "reads/writes" "SQL"
}

customer -> payments.api "makes payments" "HTTPS"
```

Dotted ids (`payments.api`) are automatic: `api` is nested under `payments`, so its fully-qualified id is `payments.api`. Use the dotted form in relationships across scopes.

## Process

```forge
process {
  repo = repository "payments-api" {
    url "https://github.com/acme/payments-api"
    system payments
  }

  trunk-based = strategy "Trunk-based" {
    trunk = branch "main" {
      protection "require-review" "require-ci"
    }
    feature = branch "feature/*" {
      branches-from trunk
      merges-into trunk
    }
  }

  payments-ci = pipeline "Payments CI" {
    triggers repo "push"
    build = stage "Build & Test" {
      step "cargo test"
    }
    deploy = stage "Deploy" {
      needs build
      environment production
      gate "manual-approval"
    }
  }
}
```

## Deployment

```forge
deployment production "Production" {
  node aws "AWS" {
    technology "Amazon Web Services"
    node us-east-1 "us-east-1" {
      node eks "EKS Cluster" {
        technology "Kubernetes 1.29"
        node api-pods "API Pods" {
          technology "3 replicas"
          instance api          // binds to the container declared in model
        }
      }
    }
  }
}
```

`instance <container-id>` is what connects infrastructure to architecture. Without it, the deployment view has nothing to show for that node.

## Views

Every view has a kind, a scope (where relevant), a key, and optional title/layout:

```forge
views {
  system-context-view payments "SystemContext" {
    include *
    auto-layout lr
    title "Payment Platform — System Context"
  }

  container-view payments "Containers" { include *; auto-layout tb }
  component-view payments.api "APIComponents" { include *; auto-layout tb }
  pipeline-view payments-ci "Pipeline" { include * }
  deployment-view production "Deployment" { include * }
  tech-stack-view "TechStack" { include * }
  branching-view trunk-based "Branching" { include * }
  data-model-view "DataModel" { include * }
  trust-boundary-view "TrustBoundaries" { include * }
  team-view "Teams" { include * }

  // Dynamic view: ordered walkthrough
  container-view payments "PaymentFlow" {
    animation {
      frame "Customer initiates payment" {
        include customer
        include payments.api
        include customer -> payments.api
        notes "..."
      }
      // more frames...
    }
  }
}
```

`include *` pulls every in-scope element; `include <id>` is explicit. Layout values: `tb` (top-bottom), `lr` (left-right).

## Preprocessor

Split a large model across files:

```forge
forge "Model" {
  !include "systems/payments.forge"
  !include "systems/orders.forge"

  views {
    !include "views/*.forge"
  }
}
```

Other directives:

- `!fragment <name> { ... }` then `!use <name>` for reusable blocks.
- `!if env(FLAG) { ... }` for conditional chunks.

## Custom lint rules (`.forge-rules`)

```forge-rules
rule no-shared-db {
  severity warning
  for container tag="database"
  expect-max-incoming 1
  message "Databases should have a single owning service"
}
```

Apply with `forge check --rules team-rules.forge-rules`. The MCP does not currently load custom rules; use the CLI when it matters.

## Editing workflow

1. **Read before writing.** If the file already exists, call `forge_overview` or `Read` on the source path — don't rewrite what's already there.
2. **Validate small edits.** Paste the edited block into `forge_validate` before saving. Errors include line and column.
3. **Reload the MCP.** After saving, `forge_reload` so subsequent queries reflect the change.
4. **Don't fight the merge pass.** If the user is running `forge analyze --merge` in CI, manual edits to `inferred:*`-tagged elements may be refreshed away. Either remove the `inferred` tag to pin the element or move the edit into a `!include`-d file that the scanner won't touch.

See `references/grammar.md` for the full kebab-case keyword list and `references/examples.md` for pattern cookbook (common shapes: multi-repo monorepo, event-driven system, Kubernetes deployment).

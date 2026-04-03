# Forge — A Unified Software Modeling DSL

## 1. Vision

Forge is a text-based modeling language and toolchain for describing the *structure* of software systems and the *processes* used to build and deliver them — from a single, coherent model. Where Structurizr focuses on architecture (C4 containers, components, deployment) and Mermaid provides ad-hoc diagram types, Forge unifies both under one semantic model so that a system's branching strategy, CI/CD pipeline, architecture, and deployment topology all reference the same elements and can be rendered as multiple views.

### Design Principles

1. **One model, many views.** Every diagram is a projection of a shared semantic graph — never a standalone drawing.
2. **Process is a first-class citizen.** Git-flow, trunk-based development, release trains, CI/CD pipelines, and incident runbooks live alongside containers and components.
3. **SVG-native, CSS-styled.** All output is clean SVG with well-named CSS classes, so users can theme diagrams with their own stylesheets.
4. **Embeddable everywhere.** Output works inline in Markdown, Hugo, MkDocs, Docusaurus, and any static-site generator.
5. **Single binary, zero dependencies.** The entire toolchain — parser, renderer, analyzer, doc generator, linter, and MCP server — ships as one statically-linked Rust binary. No JVM, no Node, no Python runtime required.
6. **AI-augmentable.** The model format is designed to be both human-writable and machine-generatable from source code, git history, and CI config. Forge exposes its capabilities as an MCP server so AI agents can query, build, and lint models programmatically.
7. **Living documentation.** `forge analyze` extracts models from real code; `forge generate` publishes them as a browsable static site; `forge generate --diff` highlights what changed. Architecture documentation stays in sync with the codebase.

---

## 2. Semantic Model

The heart of Forge is a typed, directed graph. Every node has a `kind` and a set of typed properties. Edges are named relationships. The model is divided into three domains that share a common element namespace.

### 2.1 Structure Domain (architecture)

Borrowed from C4 but extensible:

| Kind             | Description                                      |
|------------------|--------------------------------------------------|
| `person`         | Human actor or role                              |
| `system`         | A software system (internal or external)         |
| `container`      | Runtime unit — service, app, database, queue     |
| `component`      | Logical unit inside a container                  |
| `deploymentNode` | Infrastructure — server, cluster, region, pod    |
| `artifact`       | Deployable binary, image, package                |

### 2.2 Process Domain (delivery & operations)

New to Forge — models *how* software is built and shipped:

| Kind             | Description                                      |
|------------------|--------------------------------------------------|
| `repository`     | A version-control repository                     |
| `branch`         | A named branch or branch pattern (e.g. `release/*`) |
| `pipeline`       | A CI/CD pipeline                                 |
| `stage`          | A stage or job within a pipeline                 |
| `environment`    | A target environment (dev, staging, prod)        |
| `gate`           | An approval or quality gate                      |
| `runbook`        | An operational procedure                         |
| `step`           | A step within a runbook or pipeline              |

### 2.3 Flow Domain (branching & release strategies)

Models the temporal flow of code through branches — like Mermaid gitgraph but connected to the architecture model:

| Kind             | Description                                      |
|------------------|--------------------------------------------------|
| `commit`         | A point-in-time snapshot on a branch             |
| `merge`          | A merge event joining two branches               |
| `cherryPick`     | A cherry-pick of a specific commit               |
| `release`        | A release event tagged to a commit               |
| `hotfix`         | A hotfix branch/merge pattern                    |

### 2.4 Relationships

Relationships are first-class edges with a `verb`, optional `technology`, and optional `description`:

```
<source> -> <target> [verb] [technology] [description]
```

Built-in relationship verbs include `uses`, `deploysTo`, `triggers`, `mergesInto`, `branchesFrom`, `reads`, `writes`, `publishes`, `subscribes`, `approves`.

---

## 3. DSL Syntax

The DSL is designed to feel familiar to anyone who has used Structurizr or HCL (Terraform), with block-scoped definitions and minimal punctuation.

### 3.1 Workspace and Model

```forge
forge "Payment Platform" {
  description "Architecture and delivery model for Payments"

  // ───────── Structure ─────────
  model {
    customer = person "Customer" {
      description "End user making payments"
    }

    payments = system "Payment Service" {
      description "Processes card and bank payments"
      tags "core" "pci"

      api = container "Payment API" {
        technology "Rust / Actix"
        description "REST + gRPC gateway"
      }

      processor = container "Payment Processor" {
        technology "Rust"
      }

      db = container "Ledger DB" {
        technology "PostgreSQL 16"
        tags "database"
      }

      api -> processor "delegates to" "gRPC"
      processor -> db "reads/writes" "SQL"
    }

    customer -> payments.api "makes payments" "HTTPS"
  }
```

### 3.2 Process Definitions

```forge
  // ───────── Delivery Process ─────────
  process {
    repo = repository "payments-api" {
      url "https://github.com/acme/payments-api"
      system payments          // links to the structure model
    }

    // Branching strategy
    strategy "trunk-based" {
      trunk = branch "main" {
        protection "require-review" "require-ci"
      }

      feature = branch "feature/*" {
        branchesFrom trunk
        mergesInto trunk
        lifetime "short-lived"     // < 2 days recommended
      }

      rel = branch "release/*" {
        branchesFrom trunk
        tags "release"
      }
    }

    // CI/CD pipeline
    pipeline "payments-ci" {
      triggers repo.trunk on "push"
      triggers repo.feature on "pull-request"

      build = stage "Build" {
        step "cargo build --release"
        step "cargo test"
        produces artifact "payments-api:latest" {
          type "docker-image"
          deploysTo payments.api
        }
      }

      deploy_staging = stage "Deploy Staging" {
        needs build
        environment staging
        gate "integration-tests-pass"
      }

      deploy_prod = stage "Deploy Production" {
        needs deploy_staging
        environment production
        gate "manual-approval" {
          approvers "platform-team"
        }
      }
    }
  }
```

### 3.3 Flow (Git History) Descriptions

For documenting branching patterns and release flows visually:

```forge
  // ───────── Flow (like gitgraph) ─────────
  flow "release-cadence" {
    branch main
    commit id:"m1" "Initial"
    commit id:"m2" "Feature A"

    branch "release/1.0" from main
    commit id:"r1" "Prep 1.0"
    commit id:"r2" tag:"v1.0.0" "Release 1.0"

    checkout main
    commit id:"m3" "Feature B"

    branch "hotfix/1.0.1" from "release/1.0"
    commit id:"h1" "Fix CVE-1234"
    merge "release/1.0" id:"h-merge"
    cherry-pick main id:"h1"

    checkout main
    commit id:"m4" "Feature C"

    branch "release/1.1" from main
    commit id:"r3" tag:"v1.1.0" "Release 1.1"
  }
```

### 3.4 Views

Views are named projections of the model. Each view selects elements to include and can override layout, styling, and labels.

```forge
  // ───────── Views ─────────
  views {
    systemContext payments "SystemContext" {
      include *
      autoLayout lr
      title "Payment Platform — System Context"
    }

    container payments "Containers" {
      include *
      autoLayout tb
    }

    pipelineView "payments-ci" "CI-CD" {
      include *
      autoLayout lr
      title "Payment API — CI/CD Pipeline"
    }

    gitGraph "release-cadence" "BranchingStrategy" {
      orientation lr
      showCommitLabels true
      showTags true
      title "Trunk-Based Release Flow"
    }

    deploymentView production "ProdDeployment" {
      include *
      autoLayout tb
    }

    // Composite view: overlay process onto structure
    composite "FullPicture" {
      include systemContext "SystemContext"
      include pipelineView "CI-CD"
      layout grid 2
    }
  }
```

### 3.5 Styles

Styles use a CSS-like syntax that maps directly to CSS classes in the SVG output:

```forge
  // ───────── Styles ─────────
  styles {
    element "person" {
      shape person
      background "#08427B"
      color "#ffffff"
      fontSize 16
    }

    element "container" {
      shape roundedBox
      background "#438DD5"
      color "#ffffff"
    }

    element[tag="database"] {
      shape cylinder
      background "#1168BD"
    }

    relationship * {
      color "#707070"
      style dashed
      fontSize 12
    }

    branch "main" {
      color "#2E7D32"
      lineWidth 3
    }

    branch "feature/*" {
      color "#1565C0"
    }

    branch "release/*" {
      color "#E65100"
    }

    stage * {
      shape box
      background "#F5F5F5"
      border "#BDBDBD"
    }

    gate * {
      shape diamond
      background "#FFF3E0"
      border "#E65100"
    }
  }
}
```

---

## 3.6 File Composition Directives

Real-world models grow beyond a single file. Forge supports a set of `!` directives for splitting models across files, reusing shared definitions, and composing workspaces from independently maintained fragments.

### Directive Reference

| Directive                  | Scope                     | Description                                                  |
|----------------------------|---------------------------|--------------------------------------------------------------|
| `!include <path>`          | Anywhere in a block       | Inline the contents of another `.forge` file at this point   |
| `!include <glob>`          | Anywhere in a block       | Include all files matching a glob pattern                    |
| `!include <url>`           | Anywhere in a block       | Fetch and inline a remote `.forge` file (HTTPS only)         |
| `!ref <id>`                | Model, Process, Flow      | Reference an element defined in another included file        |
| `!extends <path>`          | Top-level `forge` block   | Inherit and extend another workspace                         |
| `!override`                | Inside an extended block  | Override a property or child block from the base workspace   |
| `!fragment <name> { ... }` | Top-level                 | Define a reusable named fragment (not rendered until included)|
| `!use <fragment-name>`     | Anywhere in a block       | Inline a named fragment                                      |

### Resolution Rules

1. **Include paths** are resolved relative to the file containing the directive, not the root workspace file.
2. **Glob includes** are sorted lexicographically to ensure deterministic ordering across platforms.
3. **Circular includes** are detected at parse time and produce an error with the full inclusion chain.
4. **Identifier scoping**: All identifiers share a single flat namespace within the merged workspace. Name collisions across files produce an error at merge time, unless `!override` is used in an `!extends` context.
5. **Remote includes** are cached locally (content-addressable by SHA-256) and verified on each build. The `--offline` flag uses cached versions only.

### Example: Multi-File Project Layout

```
acme-platform/
├── forge.forge              # Root workspace — imports everything
├── model/
│   ├── people.forge         # Actors and personas
│   ├── payments.forge       # Payment system structure
│   ├── catalog.forge        # Catalog system structure
│   └── shared-infra.forge   # Shared databases, queues
├── process/
│   ├── branching.forge      # Branching strategy
│   ├── ci-payments.forge    # Payment service CI/CD
│   └── ci-catalog.forge     # Catalog service CI/CD
├── flows/
│   └── release-cadence.forge
├── views/
│   ├── architecture.forge   # Structure views
│   ├── pipelines.forge      # Process views
│   └── git-flows.forge      # Flow views
├── styles/
│   ├── default.forge        # Default style definitions
│   └── dark.forge           # Dark theme overrides
└── fragments/
    ├── standard-pipeline.forge  # Reusable CI/CD pattern
    └── trunk-based.forge        # Reusable branching strategy
```

**Root workspace file (`forge.forge`):**

```forge
forge "Acme Platform" {
  description "Enterprise architecture and delivery for Acme"

  model {
    !include model/people.forge
    !include model/*.forge          // glob — includes all model files
  }

  process {
    !include process/branching.forge
    !include process/ci-*.forge     // all CI pipeline definitions
  }

  flow {
    !include flows/*.forge
  }

  views {
    !include views/*.forge
  }

  styles {
    !include styles/default.forge
  }
}
```

**Reusable fragment (`fragments/standard-pipeline.forge`):**

```forge
!fragment standard-pipeline {
  // A parameterised CI/CD pattern reused across services.
  // The enclosing context must define: $repo, $service, $env

  test = stage "Test" {
    step "cargo test --workspace"
    step "cargo clippy -- -D warnings"
  }

  build = stage "Build" {
    needs test
    step "docker build -t $service:$SHA ."
    produces artifact "$service:latest" {
      type "docker-image"
    }
  }

  deploy = stage "Deploy" {
    needs build
    environment $env
    gate "smoke-tests-pass"
  }
}
```

**Using the fragment:**

```forge
// process/ci-payments.forge
pipeline "payments-ci" {
  triggers repo.main on "push"

  !use standard-pipeline   // inlines the fragment here

  // Extend with service-specific stages
  security_scan = stage "Security Scan" {
    needs build
    step "trivy image payments-api:latest"
  }
}
```

### Extending Workspaces

For organisations that maintain a base architecture template:

```forge
// team-payments/forge.forge
forge "Payments Team" {
  !extends ../../platform/forge.forge   // inherit the base model

  model {
    // Add team-specific elements — base elements are still available
    fraud = container "Fraud Detector" {
      technology "Python / scikit-learn"
    }

    !override payments.api {
      // Override a property defined in the base workspace
      description "REST + gRPC gateway (v2 with fraud screening)"
    }

    payments.api -> fraud "screens transactions" "gRPC"
  }

  views {
    container payments "PaymentsDetail" {
      include *
      include fraud
      autoLayout tb
    }
  }
}
```

### Conditional Includes

For environment-specific or feature-flagged model sections:

```forge
model {
  !include model/core.forge

  !if env("FORGE_ENV") == "production" {
    !include model/prod-only.forge
  }

  !if feature("threat-modeling") {
    !include model/threats.forge
  }
}
```

---

## 3.7 Animation

Forge supports animation as a first-class view property, allowing diagrams to reveal information step-by-step. This is essential for walkthroughs, presentations, architecture decision explanations, and depicting temporal sequences like deployment rollouts or data flow through a pipeline.

### Animation Model

Animation is defined using ordered **frames** within a view. Each frame specifies which elements and relationships become visible (or change state) at that step. The renderer produces SVG with embedded CSS animations or optional JavaScript-free SMIL, keeping the output self-contained and embeddable.

### DSL Syntax

```forge
views {
  // Animated architecture walkthrough
  container payments "PaymentsWalkthrough" {
    title "Payment Flow — Step by Step"
    autoLayout tb

    animation {
      // Frame 1: Show the customer and web gateway
      frame "Customer arrives" {
        include customer
        include payments.api
        include customer -> payments.api
      }

      // Frame 2: API delegates to processor
      frame "Payment processing" {
        include payments.processor
        include payments.api -> payments.processor
      }

      // Frame 3: Processor writes to ledger
      frame "Record transaction" {
        include payments.db
        include payments.processor -> payments.db
      }

      // Frame 4: Everything visible — highlight the critical path
      frame "Complete flow" {
        include *
        highlight payments.api -> payments.processor -> payments.db {
          color "#E65100"
          lineWidth 3
          label "Critical path"
        }
      }
    }
  }

  // Animated pipeline execution
  pipelineView "payments-ci" "PipelineAnimation" {
    autoLayout lr

    animation {
      frame "Trigger" {
        include repo.main
        state repo.main "push event"
      }

      frame "Build & Test" {
        include build
        state build "running" {
          pulse true           // animated pulsing effect
          color "#FFA726"
        }
      }

      frame "Build passes" {
        state build "passed" {
          color "#66BB6A"
          icon "check"
        }
      }

      frame "Deploy to staging" {
        include deploy_staging
        state deploy_staging "running" { pulse true }
      }

      frame "Awaiting approval" {
        include deploy_prod
        state deploy_prod.gate "waiting" {
          pulse true
          color "#FFA726"
        }
      }

      frame "Production deployed" {
        state deploy_prod "passed" { color "#66BB6A" }
      }
    }
  }

  // Animated git flow
  gitGraph "release-cadence" "AnimatedRelease" {
    orientation lr
    showTags true

    animation {
      // Each frame reveals commits progressively
      frame "Development" {
        include main[..m2]           // commits up to m2
      }

      frame "Release branch" {
        include "release/1.0"[..r2]
      }

      frame "Hotfix" {
        include "hotfix/1.0.1"[*]
        highlight h1 { color "#D32F2F" }
      }

      frame "Continue development" {
        include main[m3..m4]
      }

      frame "Next release" {
        include *                    // reveal everything
      }
    }
  }
}
```

### Animation Properties

| Property           | Type           | Description                                              |
|--------------------|----------------|----------------------------------------------------------|
| `frame <label>`    | Block          | A named step in the animation sequence                   |
| `include`          | Element/Rel    | Make elements visible at this frame                      |
| `exclude`          | Element/Rel    | Hide elements at this frame                              |
| `highlight`        | Element/Rel    | Apply emphasis styling to a path or element              |
| `state`            | Element + label| Change the displayed state of an element                 |
| `pulse`            | Boolean        | Apply a pulsing animation effect                         |
| `transition`       | Duration       | Time between auto-advancing frames (default: manual)     |
| `easing`           | Keyword        | CSS easing function: `ease`, `linear`, `ease-in-out`     |
| `fadeIn`           | Duration       | Fade-in duration for newly visible elements              |
| `moveFrom`         | Direction      | Slide-in direction: `left`, `right`, `top`, `bottom`     |

### SVG Animation Output

The renderer produces self-contained SVG using CSS keyframes. No JavaScript is required for basic playback — the animation runs automatically or can be controlled via CSS classes that a host page toggles.

```xml
<svg class="forge-diagram forge-animated" data-frames="6" data-current="0">
  <defs>
    <style>
      /* Frame visibility — controlled by data-current attribute or CSS */
      .forge-frame { opacity: 0; transition: opacity 0.4s ease-in-out; }
      .forge-frame--active { opacity: 1; }

      /* Pulse effect */
      @keyframes forge-pulse {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.5; }
      }
      .forge-pulse { animation: forge-pulse 1.5s ease-in-out infinite; }

      /* Fade-in for new elements */
      @keyframes forge-fade-in {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
      }
      .forge-enter { animation: forge-fade-in 0.4s ease-out forwards; }

      /* Highlight glow */
      .forge-highlight { filter: drop-shadow(0 0 6px #E65100); }
    </style>
  </defs>

  <!-- Frame 1 -->
  <g class="forge-frame" data-frame="0">
    <g class="forge-element forge-element--person forge-enter" ...>
      <!-- Customer -->
    </g>
    <g class="forge-element forge-element--container forge-enter" ...>
      <!-- Payment API -->
    </g>
  </g>

  <!-- Frame 2 — cumulative: previous frames remain visible -->
  <g class="forge-frame" data-frame="1">
    <g class="forge-element forge-element--container forge-enter" ...>
      <!-- Payment Processor -->
    </g>
    <path class="forge-relationship forge-enter" .../>
  </g>

  <!-- Navigation hint (optional, CSS-only) -->
  <g class="forge-frame-indicator">
    <circle class="forge-dot" data-frame="0" cx="10" cy="10" r="4" />
    <circle class="forge-dot" data-frame="1" cx="25" cy="10" r="4" />
    <circle class="forge-dot" data-frame="2" cx="40" cy="10" r="4" />
  </g>
</svg>
```

### Playback Control

Since the SVG output is JavaScript-free, playback is controlled by the host environment:

**Static (print/PDF):** All frames rendered simultaneously — the final state of the animation is shown.

**Markdown/Hugo/MkDocs:** The site-generator plugin injects a minimal `<script>` (< 1KB) that listens for keyboard arrows or click events and toggles `data-current` on the SVG root. This script is optional — without it, the diagram shows the final state.

**Presentation mode:** `forge serve --present` opens a local viewer that supports keyboard navigation (arrow keys, spacebar) through frames, with a frame counter and optional speaker notes.

```forge
// Optional: attach speaker notes to frames
animation {
  frame "Customer arrives" {
    include customer, payments.api
    notes "The customer initiates a payment via the web gateway.
           Note the HTTPS connection — all traffic is encrypted in transit."
  }
}
```

### Animation Rendering Modes

The `--animate` flag controls how animation is rendered:

```
forge build --animate css       # CSS keyframes (default) — self-contained SVG
forge build --animate smil      # SMIL animation — wider SVG viewer support
forge build --animate none      # Static final-state — for print/PDF
forge build --animate gif       # Render as animated GIF (via resvg + gifski)
forge build --animate webm      # Render as WebM video
```

---

## 4. Architecture

### 4.1 Single Binary Design

Forge ships as a single statically-linked Rust binary (`forge`) containing every capability. No language runtimes, no external dependencies, no plugins to install. Copy the binary to your PATH and you have the full toolchain.

```
┌──────────────────────────────────────────────────────────────────┐
│                       forge (single Rust binary)                 │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────┐  │
│  │  Parser   │  │  Model   │  │ Analyzer  │  │   Renderer     │  │
│  │ (pest/   │─▶│  Graph   │◀─│ (code,git │  │  (SVG writer)  │  │
│  │  nom)    │  │          │  │  CI,k8s)  │  │               │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────────────┘  │
│                      │              │              │              │
│                      ▼              ▼              ▼              │
│               ┌──────────┐  ┌───────────┐  ┌───────────────┐    │
│               │  Checker  │  │  Layout   │  │  Site Gen     │    │
│               │ (linter)  │  │  Engine   │  │ (HTML/static) │    │
│               └──────────┘  └───────────┘  └───────────────┘    │
│                                     │              │              │
│                              ┌──────────────┐ ┌──────────┐      │
│                              │  CSS Theming  │ │ MCP Srvr │      │
│                              └──────────────┘ └──────────┘      │
└──────────────────────────────────────────────────────────────────┘
        │              │              │              │
        ▼              ▼              ▼              ▼
   .forge files   model.json     *.svg output   _site/ (static)
```

### 4.2 Crate / Module Breakdown

All modules compile into one binary via Cargo workspace with a single `[[bin]]` target. Feature flags control optional heavyweight dependencies (e.g. `resvg` for PNG export, `tower` for MCP server).

| Module            | Crate              | Responsibility                                                  |
|-------------------|--------------------|-----------------------------------------------------------------|
| `forge-parser`    | `forge-parser`     | PEG/packrat parser (pest) for `.forge` files → AST; handles `!include`, `!extends`, `!fragment`, `!use`, `!if`, globs, and cycle detection |
| `forge-model`     | `forge-model`      | Semantic graph: nodes, edges, properties, validation, queries   |
| `forge-analyze`   | `forge-analyze`    | Codebase scanners: Rust/Go/TS/Java source → components; git log → flow; GitHub Actions / GitLab CI → pipelines; Dockerfiles → containers; K8s manifests → deployment nodes; OpenAPI → API containers. All scanners produce `.forge` fragments or merge directly into a `Model` |
| `forge-check`     | `forge-check`      | Architectural linter: rule engine with built-in and custom rules; detects cycles, orphaned elements, missing descriptions, overly-coupled components, security anti-patterns, naming violations |
| `forge-layout`    | `forge-layout`     | Auto-layout algorithms — layered (Sugiyama), force-directed, grid, pipeline |
| `forge-render`    | `forge-render`     | Model → SVG with CSS class annotations; animation frame generation (CSS keyframes, SMIL, GIF/WebM export); filled and outline rendering modes |
| `forge-sitegen`   | `forge-sitegen`    | Static documentation site generator: model → multi-page HTML with navigation, search index, embedded SVGs, and architectural diff overlays |
| `forge-diff`      | `forge-diff`       | Model differencing engine: compares two model snapshots (or git revisions) and produces a typed changeset (added/removed/modified elements and relationships) |
| `forge-mcp`       | `forge-mcp`        | MCP (Model Context Protocol) server: exposes all Forge capabilities as tools for AI agents — query model, render views, run checks, analyze code, generate diffs |
| `forge-cli`       | `forge-cli`        | CLI entry point: `forge build`, `forge analyze`, `forge generate`, `forge check`, `forge mcp`, `forge watch`, `forge export` |
| `forge-lsp`       | `forge-lsp`        | Language Server Protocol for editor integration                 |

### 4.3 Data Flow

```
                          ┌───────────────────────────────────────────────┐
                          │            forge analyze                       │
 Source Code ──┐          │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │
 Git History ──┤          │  │ code   │ │ git    │ │ CI     │ │ k8s/   │ │
 CI Config  ───┼────────▶ │  │scanner │ │scanner │ │scanner │ │docker  │ │
 K8s / Docker──┤          │  └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘ │
 OpenAPI ──────┘          │      └──────────┴──────────┴──────────┘      │
                          └──────────────────────┬────────────────────────┘
                                                 │
                                                 ▼
 Manual Edit ──────────────────────────▶  ┌───────────┐
                                          │  .forge   │
                                          │  files    │
                                          └─────┬─────┘
                                                │
                                          ┌─────▼─────┐
                                          │  Parser   │
                                          └─────┬─────┘
                                                │
                          ┌─────────────────────▼──────────────────────┐
                          │              Model Graph                    │
                          │                                            │
                          │  ◄── forge check (lint rules)              │
                          │  ◄── forge diff (compare revisions)        │
                          └──┬────────────┬────────────┬───────────────┘
                             │            │            │
                        ┌────▼────┐ ┌─────▼──────┐ ┌──▼──────────┐
                        │  JSON   │ │   Views    │ │  Site Gen   │
                        │ export  │ │ (filtered  │ │  (forge     │
                        └─────────┘ │  subgraphs)│ │  generate)  │
                                    └─────┬──────┘ └──────┬──────┘
                                          │               │
                                    ┌─────▼──────┐  ┌─────▼──────┐
                                    │   Layout   │  │  _site/    │
                                    │   Engine   │  │  static    │
                                    └─────┬──────┘  │  HTML docs │
                                          │         └────────────┘
                                    ┌─────▼──────┐
                                    │  SVG + CSS │──▶ embed in Markdown
                                    │  Render    │
                                    └────────────┘
                                          │
                                    ┌─────▼──────┐
                                    │  MCP Srvr  │──▶ AI agent access
                                    │  (forge    │    (query, render,
                                    │   mcp)     │     check, analyze)
                                    └────────────┘
```

---

## 5. SVG Output & CSS Styling

### 5.1 SVG Structure

Every rendered SVG uses semantic CSS classes so users can restyle without modifying the tool:

```xml
<svg xmlns="http://www.w3.org/2000/svg" class="forge-diagram forge-view-container">
  <style>
    /* Default theme — users override with their own stylesheet */
    .forge-element { stroke: #333; stroke-width: 1; }
    .forge-element--person { fill: #08427B; }
    .forge-element--container { fill: #438DD5; }
    .forge-element--database { fill: #1168BD; }
    .forge-relationship { stroke: #707070; stroke-dasharray: 5,5; }
    .forge-label { font-family: system-ui, sans-serif; font-size: 14px; }
    .forge-branch--main { stroke: #2E7D32; stroke-width: 3; }
    .forge-commit { fill: #333; r: 6; }
    .forge-tag { fill: #E65100; font-size: 11px; }
    .forge-stage { fill: #F5F5F5; stroke: #BDBDBD; rx: 4; }
    .forge-gate { fill: #FFF3E0; stroke: #E65100; }
  </style>

  <g class="forge-elements">
    <g class="forge-element forge-element--container" data-id="payments.api">
      <rect ... />
      <text class="forge-label">Payment API</text>
      <text class="forge-label forge-label--technology">Rust / Actix</text>
    </g>
    <!-- ... more elements ... -->
  </g>

  <g class="forge-relationships">
    <path class="forge-relationship" data-from="payments.api" data-to="payments.processor" ... />
    <text class="forge-label forge-label--relationship">delegates to [gRPC]</text>
  </g>
</svg>
```

### 5.2 CSS Override Example

Users drop a `forge-theme.css` alongside their docs:

```css
/* Dark theme override */
.forge-diagram { background: #1a1a2e; }
.forge-element--container { fill: #16213e; stroke: #0f3460; }
.forge-element--person { fill: #533483; }
.forge-label { fill: #e0e0e0; }
.forge-relationship { stroke: #555; }
.forge-branch--main { stroke: #66bb6a; }
```

---

## 6. Site Generator Integration

### 6.1 Markdown Embedding

Forge diagrams embed in Markdown using fenced code blocks:

````markdown
# System Architecture

The payment platform is structured as follows:

```forge-view SystemContext
source: ./architecture.forge
theme: ./dark-theme.css
width: 800
```

## Branching Strategy

Our release cadence follows trunk-based development:

```forge-view BranchingStrategy
source: ./architecture.forge
```
````

### 6.2 Hugo Integration

A Hugo render hook or shortcode:

```html
<!-- layouts/shortcodes/forge.html -->
{{ $view := .Get "view" }}
{{ $source := .Get "source" | default "architecture.forge" }}
{{ $svg := printf "forge build --view %s --source %s --format svg-inline" $view $source }}
{{ $svg | safeHTML }}
```

Or as a Hugo module that hooks into the build:

```toml
# config.toml
[module]
  [[module.imports]]
    path = "github.com/acme/forge-hugo"

[params.forge]
  source = "docs/architecture.forge"
  theme = "docs/forge-theme.css"
```

### 6.3 MkDocs Integration

```yaml
# mkdocs.yml
plugins:
  - forge:
      source: docs/architecture.forge
      theme: docs/forge-theme.css
      views:
        - SystemContext
        - Containers
        - BranchingStrategy
        - CI-CD
```

The plugin calls `forge build` at build time and injects inline SVG into the rendered HTML.

---

## 7. AI Integration

### 7.1 `forge analyze` as the AI Bridge

Rather than a separate `forge ai` command, Forge's AI capabilities are built into `forge analyze` and exposed via the MCP server (§8.6). The analyze command's scanners are deterministic (AST parsing, config parsing), but the results can be enriched by AI agents using the MCP tools.

Typical AI-assisted workflow:

```bash
# 1. Deterministic scan produces a baseline model
forge analyze --out baseline.forge ./src

# 2. AI agent (via MCP) reviews the model and fills gaps
#    e.g. "Describe the purpose of each container based on its code"
#    The agent calls forge_query to list elements, reads source files,
#    then calls forge_suggest_fix to propose description additions.

# 3. Human reviews and confirms
forge check --inferred    # shows all AI-inferred elements
```

### 7.2 MCP-Powered AI Augmentation

With the MCP server running, an AI agent can:

- Query the model to understand current architecture
- Analyze new codebases and propose model additions
- Run checks and suggest fixes for violations
- Generate diffs to explain what changed between releases
- Render views to include in documentation or conversations

The MCP interface makes Forge accessible to any AI agent that speaks MCP — Claude Code, Cursor, Windsurf, or custom agents built on the Agent SDK. See §8.6 for the full tool list.

### 7.3 Confidence Scoring

Elements inferred by `forge analyze` carry a confidence score (0.0–1.0). The score reflects how certain the scanner is about the element's existence and properties:

| Confidence | Meaning                          | Visual Treatment               |
|------------|----------------------------------|--------------------------------|
| 1.0        | Explicit in source               | Normal rendering               |
| 0.7–0.99   | High confidence inference        | Normal + `inferred` tag        |
| 0.5–0.69   | Medium confidence                | Dashed border + `inferred` tag |
| < 0.5      | Low confidence (filtered by default) | Not included unless `--confidence 0` |

Run `forge check --inferred` to list all unconfirmed elements. Manually editing an inferred element in a `.forge` file removes the `inferred` tag and sets confidence to 1.0.

---

## 8. CLI Interface

All commands are subcommands of the single `forge` binary.

```
forge — A unified software modeling tool

USAGE:
    forge <COMMAND> [OPTIONS]

COMMANDS:
    analyze     Scan codebases and generate a .forge model automatically
    build       Parse .forge files and render views to SVG
    generate    Produce a static documentation website from a model
    check       Lint and validate a model against architectural rules
    watch       Watch for changes and rebuild incrementally
    export      Export model as JSON, YAML, or Structurizr DSL
    import      Import from Structurizr DSL, PlantUML, or Mermaid
    serve       Start a local preview server with live reload
    mcp         Start the MCP (Model Context Protocol) server
    lsp         Start the Language Server Protocol server
```

### 8.1 `forge analyze`

Scans one or more codebases and produces a `.forge` model describing everything found: source components, dependencies, git branching patterns, CI/CD pipelines, deployment topology, and API surfaces.

```
forge analyze [OPTIONS] [PATH...]

ARGUMENTS:
    [PATH...]              One or more directories to scan (default: .)

OPTIONS:
    --out <FILE>           Output .forge file (default: ./forge.forge)
    --merge <FILE>         Merge results into an existing .forge model
    --scanners <LIST>      Comma-separated scanner list (default: all)
                           Available: code, git, ci, k8s, docker, openapi
    --lang <LIST>          Limit code scanner to these languages
                           Available: rust, go, typescript, java, python, csharp
    --depth <N>            Max directory depth to scan (default: unlimited)
    --exclude <GLOB>       Exclude paths matching glob (repeatable)
    --confidence <FLOAT>   Min confidence threshold for inferred elements
                           (default: 0.5, range 0.0–1.0)
    --dry-run              Show what would be generated without writing

SCANNERS:
    code       Parses source files to discover components, modules, and
               dependency relationships. Uses tree-sitter for language-
               agnostic AST analysis. Detects import graphs, service
               boundaries, and public API surfaces.

    git        Reads git log and branch refs to infer branching strategy
               (trunk-based, git-flow, github-flow), release patterns,
               contributor activity, and code ownership.

    ci         Parses CI/CD configuration files:
               - GitHub Actions (.github/workflows/*.yml)
               - GitLab CI (.gitlab-ci.yml)
               - Jenkins (Jenkinsfile)
               - CircleCI (.circleci/config.yml)
               Produces pipeline, stage, and gate elements.

    k8s        Parses Kubernetes manifests (Deployments, Services,
               Ingresses, ConfigMaps) → deployment nodes, networking
               relationships, and environment definitions.

    docker     Parses Dockerfiles and docker-compose.yml →
               container elements with technology tags, networking,
               and volume relationships.

    openapi    Parses OpenAPI/Swagger specs → API container elements
               with endpoint descriptions and inter-service relationships.

EXAMPLES:
    forge analyze                                   # scan current dir, all scanners
    forge analyze ./payments ./catalog              # scan multiple repos
    forge analyze --scanners code,ci --lang rust    # only code + CI, Rust only
    forge analyze --merge architecture.forge        # add to existing model
    forge analyze --out payments.forge ./payments   # write to specific file
```

When scanning multiple directories, Forge treats each top-level path as a candidate software system and infers inter-system relationships from shared dependencies, API calls, and queue/topic references.

Elements inferred with confidence below 1.0 are tagged `inferred` and render with a dashed border by default. Run `forge check --inferred` to list all unconfirmed elements.

### 8.2 `forge build`

```
forge build [OPTIONS]

OPTIONS:
    --source <FILE>        Input .forge file (default: ./forge.forge)
    --view <NAME>          Render a specific view (default: all)
    --format <FORMAT>      Output format: svg, svg-inline, png, pdf
    --style <MODE>         Rendering style: filled (default) or outline
    --theme <FILE>         CSS theme file to apply
    --out <DIR>            Output directory (default: ./forge-output/)
    --jobs <N>             Parallel rendering jobs (default: CPU count)
    --animate <MODE>       Animation mode: css, smil, none, gif, webm

EXAMPLES:
    forge build
    forge build --view SystemContext --format svg
    forge build --style outline --theme dark.css
```

### 8.3 `forge generate`

Produces a complete static documentation website from a Forge model. The generated site includes navigable architecture diagrams, element detail pages, relationship maps, and an embedded search index. Ready to deploy to GitHub Pages, Netlify, S3, or any static host.

```
forge generate [OPTIONS]

OPTIONS:
    --source <FILE>        Input .forge file (default: ./forge.forge)
    --out <DIR>            Output directory (default: ./_site/)
    --title <STRING>       Site title (default: model name)
    --base-url <URL>       Base URL for deployment (default: /)
    --theme <FILE>         CSS theme file
    --style <MODE>         Diagram rendering style: filled or outline
    --template <DIR>       Custom HTML templates directory
    --diff <REF>           Git ref or .forge file to diff against (see §8.4)
    --no-search            Disable search index generation
    --no-source            Don't embed .forge source in the site
    --watch                Rebuild on file changes

GENERATED SITE STRUCTURE:
    _site/
    ├── index.html              # Landing page with model overview
    ├── views/
    │   ├── SystemContext.html   # One page per view, with embedded SVG
    │   ├── Containers.html
    │   └── Pipeline.html
    ├── elements/
    │   ├── payments-api.html   # Detail page per element
    │   ├── ledger-db.html      # Shows relationships, properties, tags
    │   └── ...
    ├── checks/
    │   └── report.html         # Architectural check results (if issues found)
    ├── diff/
    │   └── index.html          # Architectural diff (if --diff was used)
    ├── search-index.json       # Client-side search index
    ├── assets/
    │   ├── forge.css            # Default site styles
    │   ├── forge.js             # Minimal JS (search, animation playback)
    │   └── diagrams/            # Rendered SVGs
    └── forge.json              # Machine-readable model export

EXAMPLES:
    forge generate                                        # default site
    forge generate --out docs/_site --base-url /arch/     # for GitHub Pages
    forge generate --diff HEAD~5                          # highlight recent changes
    forge generate --diff v1.2.0                          # diff against tagged release
    forge generate --template ./my-templates              # custom look & feel
```

### 8.4 Architectural Diff (`--diff`)

The `--diff` flag computes the difference between the current model and a previous version, then overlays change indicators on every diagram and generates a dedicated diff report page.

```
forge generate --diff <REF>

REF can be:
    HEAD~N          Git ref — Forge checks out the .forge files at that
                    revision, parses them, and diffs against the current model
    <tag>           Git tag (e.g. v1.2.0)
    <commit-sha>    Specific commit
    <path.forge>    Path to a previous .forge snapshot file
```

The diff engine produces a typed changeset:

| Change Type     | Visual Indicator                     | CSS Class                   |
|-----------------|--------------------------------------|-----------------------------|
| Added element   | Green dashed border + "NEW" badge    | `.forge-diff--added`        |
| Removed element | Red dashed border + strikethrough    | `.forge-diff--removed`      |
| Modified element| Amber border + delta icon            | `.forge-diff--modified`     |
| Added relationship | Green dashed line               | `.forge-diff-rel--added`    |
| Removed relationship | Red dashed line + strikethrough | `.forge-diff-rel--removed` |
| Modified relationship | Amber dashed line             | `.forge-diff-rel--modified` |

The diff report page (`_site/diff/index.html`) provides a summary table of all changes, filterable by element kind, and side-by-side "before/after" diagram views.

Diff also works standalone without site generation:

```bash
forge diff <old.forge> <new.forge>              # compare two files
forge diff --ref HEAD~5                          # compare against git history
forge diff --ref v1.0.0 --format json            # machine-readable changeset
forge diff --ref v1.0.0 --format svg             # SVGs with diff overlay only
```

### 8.5 `forge check`

Analyzes a Forge model against a configurable set of architectural rules and reports violations. Think of it as a linter for software architecture.

```
forge check [OPTIONS]

OPTIONS:
    --source <FILE>        Input .forge file (default: ./forge.forge)
    --rules <FILE>         Custom rules file (default: built-in rules)
    --severity <LEVEL>     Minimum severity to report: error, warning, info
    --format <FORMAT>      Output format: text (default), json, sarif, markdown
    --fix                  Auto-fix issues where possible
    --inferred             List all elements tagged 'inferred' (from analyze)

EXIT CODES:
    0    No issues found
    1    Warnings found (with --severity warning or lower)
    2    Errors found

BUILT-IN RULES:
    dependency-cycles      Detect circular dependencies between containers/
                           components (configurable max depth)
    orphaned-elements      Elements with no relationships (may indicate
                           incomplete modeling)
    missing-descriptions   Elements or relationships lacking descriptions
    missing-technology     Containers without a technology tag
    database-direct-access Persons or external systems accessing databases
                           directly (bypassing service layer)
    single-point-failure   Containers with fan-in > threshold and no redundancy
    chatty-coupling        Pair of elements with > N relationships between them
                           (suggests they should be merged or have an API)
    naming-conventions     Element IDs and names checked against configurable
                           patterns (e.g. kebab-case, PascalCase)
    gate-coverage          Pipeline stages deploying to production without
                           a quality gate
    stale-inferred         Inferred elements older than N days without
                           human confirmation
    boundary-violation     Components in one container directly depending on
                           components in another (should go through container
                           interface)
    empty-views            Views that include no elements

CUSTOM RULES:
    Custom rules are defined in a .forge-rules file using a declarative syntax:

    ```forge-rules
    rule "max-container-coupling" {
      description "No container should depend on more than 5 others"
      severity error
      scope container
      condition count(outgoing_relationships) > 5
      message "{element.name} has {count} outgoing dependencies (max 5)"
    }

    rule "require-https" {
      description "All external-facing relationships must use HTTPS"
      severity error
      scope relationship
      condition source.kind == "person" && technology != "HTTPS"
      message "Relationship from {source.name} uses {technology}, expected HTTPS"
    }
    ```

EXAMPLES:
    forge check                                  # run all rules
    forge check --severity error                 # errors only
    forge check --format sarif > results.sarif   # for CI integration
    forge check --rules ./team-rules.forge-rules # custom rules
    forge check --inferred                       # list unconfirmed elements
    forge check --fix                            # auto-fix what's possible
```

The `--format sarif` output integrates with GitHub Code Scanning, VS Code SARIF Viewer, and other SARIF-compatible tools, so architectural violations can appear alongside code issues in your CI pipeline.

### 8.6 `forge mcp` — MCP Server

Forge exposes its full capability set as an MCP (Model Context Protocol) server, enabling AI agents and IDE extensions to interact with architectural models programmatically.

```
forge mcp [OPTIONS]

OPTIONS:
    --source <FILE>        Input .forge file (default: ./forge.forge)
    --transport <MODE>     Transport: stdio (default), http
    --port <PORT>          HTTP port (default: 3100, only with --transport http)
    --allow-write          Allow tools that modify .forge files (default: read-only)

EXAMPLES:
    forge mcp                                  # stdio transport for Claude Code
    forge mcp --transport http --port 3100     # HTTP for remote agents
    forge mcp --allow-write                    # enable analyze/fix tools
```

The MCP server exposes the following tools:

| Tool                  | Description                                           | Read/Write |
|-----------------------|-------------------------------------------------------|------------|
| `forge_query`         | Query the model graph: list elements, filter by kind/tag, find relationships, resolve paths | Read |
| `forge_render`        | Render a specific view to SVG (returns SVG string)    | Read       |
| `forge_check`         | Run architectural rules and return violations         | Read       |
| `forge_diff`          | Compare current model against a git ref or snapshot   | Read       |
| `forge_analyze`       | Scan a codebase and return discovered elements        | Write      |
| `forge_element_detail`| Get full details for a specific element (properties, relationships, views it appears in) | Read |
| `forge_search`        | Full-text search across element names, descriptions, and technologies | Read |
| `forge_validate`      | Parse and validate a .forge snippet without persisting | Read       |
| `forge_suggest_fix`   | Given a check violation, suggest a model fix          | Read       |

MCP server configuration for Claude Code (`.claude/settings.json`):

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge",
      "args": ["mcp", "--source", "./architecture.forge", "--allow-write"]
    }
  }
}
```

This enables interactions like:
- "What containers does the Payment API depend on?"
- "Show me the system context diagram"
- "Are there any architectural violations in the current model?"
- "Analyze the ./services directory and add any new services to the model"
- "What changed architecturally since the v2.0 release?"

### 8.7 Other Commands

```
forge watch [OPTIONS]       Watch for changes and rebuild incrementally
forge export [OPTIONS]      Export model as JSON, YAML, or Structurizr DSL
forge import [OPTIONS]      Import from Structurizr DSL, PlantUML, or Mermaid
forge serve [OPTIONS]       Start a local preview server with live reload
forge lsp                   Start the Language Server Protocol server
```

---

## 9. View Types

| View Type          | Description                                        | Layout    |
|--------------------|----------------------------------------------------|-----------|
| `systemContext`    | C4 Level 1 — systems and actors                    | Force/LR  |
| `container`        | C4 Level 2 — containers within a system            | Layered   |
| `component`        | C4 Level 3 — components within a container         | Layered   |
| `deployment`       | Infrastructure topology                            | Layered   |
| `gitGraph`         | Branch/commit/merge timeline                       | LR or TB  |
| `pipelineView`     | CI/CD stages, gates, and artifacts                 | LR        |
| `flowView`         | Sequence of operations in a runbook                | TB        |
| `composite`        | Grid of multiple views on one canvas               | Grid      |
| `dynamic`          | Animated sequence (numbered relationship ordering) | Layered   |
| `landscape`        | All systems in the enterprise                      | Force     |

---

## 10. Implementation Roadmap

### Phase 1 — Foundation (Months 1–3)

- Cargo workspace with single binary target, feature-flag architecture
- `forge-parser`: PEG grammar for the full DSL using `pest`
- **File composition**: `!include` (path + glob), `!fragment` / `!use`, circular-include detection
- `forge-model`: In-memory graph with validation
- `forge-render`: SVG output for structure views (systemContext, container, component); filled and outline modes
- `forge-cli`: `build` command with `--style`, `--format`, `--theme` flags
- Basic auto-layout (layered / Sugiyama)

### Phase 2 — Analyze & Check (Months 3–5)

- `forge-analyze`: code scanner (tree-sitter for Rust, Go, TS, Java, Python, C#), git scanner (gix), CI scanner (GitHub Actions, GitLab CI)
- Confidence scoring and `inferred` tagging for analyzed elements
- `forge-check`: rule engine with built-in rules (dependency cycles, orphaned elements, missing descriptions, gate coverage, boundary violations)
- Custom rule syntax (`.forge-rules` files)
- SARIF output for CI integration
- Process domain: pipeline, stage, gate, environment
- `pipelineView` view type

### Phase 3 — Generate & Diff (Months 5–7)

- `forge-sitegen`: static documentation site generator with Tera templates
- Element detail pages, view pages, navigation, client-side search (tantivy-generated index)
- `forge-diff`: model differencing engine — compare two model snapshots or git revisions
- Diff overlay rendering (added/removed/modified CSS classes on SVG elements)
- `forge generate --diff` integration — diff report page and annotated diagrams
- Flow domain: gitgraph rendering (branch, commit, merge, tag)
- **Animation engine**: frame-based animation with CSS keyframes output
- CSS theming system
- `forge watch` with incremental rebuild

### Phase 4 — MCP & AI (Months 7–9)

- `forge-mcp`: MCP server with stdio and HTTP transports
- Full tool suite: query, render, check, diff, analyze, search, validate, suggest-fix
- Docker/K8s/OpenAPI scanners for `forge analyze`
- `!extends` / `!override` for workspace inheritance
- `!include <url>` for remote includes with content-addressable caching
- `!if` conditional includes

### Phase 5 — Integration & Polish (Months 9–12)

- Hugo and MkDocs integration guides (both call `forge build` as subprocess)
- `forge serve` with live reload and `--present` mode for animated walkthroughs
- Import/export: Structurizr DSL, PlantUML C4, Mermaid
- Deployment views
- `forge-lsp` for VS Code / Neovim
- SMIL animation output mode
- PNG/PDF export via `resvg`
- Animated GIF and WebM export (via `resvg` + `gifski`)
- Force-directed layout for landscape views
- Performance optimization (target: 10k-element models in < 1s)
- Documentation and example gallery
- Cross-compilation and release automation (Linux musl, macOS universal, Windows MSVC)

### Phase 6 — Complete Modeling (Future)

Additional model dimensions for a comprehensive architecture picture:

**Data & Integration**
- **Data model**: `dataModel` block with entities, fields (name, type, constraints), and relationships (1:1, 1:N, N:M). `dataModelView` renders as an ER diagram. Entities are linked to owning containers.
- **API catalog**: `api` block per container with endpoints (method, path, payload). API surface view showing contracts between systems.
- **Event/message flows**: `eventFlow` block defining topics, queues, publishers, subscribers. Flow view showing async communication paths at runtime.

**Runtime & Operations**
- **Environment configuration**: `config` block per environment with feature flags, config values, and secret references. Shows what differs between staging and prod.
- **SLA/SLO definitions**: `slo` block per container with latency, availability, and error budget targets. Ties back to quality attributes in docs.
- **Dependency health**: `dependency` block for external systems, third-party APIs, and SaaS dependencies with criticality ratings.

**Security & Governance**
- **Trust boundaries**: `trustBoundary` blocks that group containers into security zones (public, DMZ, internal, PCI scope). Rendered as colored boundary regions on container views. Boundary-crossing relationships are highlighted.
- **Data classification**: PII, financial, public tags per data store with visual indicators.

**Team & Ownership**
- **Team ownership map**: `teams` block with team definitions and ownership assignments to containers, pipelines, and repositories. `teamView` renders an ownership grid.
- **On-call/runbook links**: Operational context per container linking to external runbook systems.

---

## 11. Technology Choices

All technology lives inside a single Rust binary. No Go, Python, or JVM runtimes.

| Concern              | Choice              | Rationale                                              |
|----------------------|---------------------|--------------------------------------------------------|
| Core language        | Rust                | Performance, safety, single static binary, no runtime  |
| Parser               | pest (PEG)          | Fast, readable grammar files, good error messages      |
| Source analysis      | tree-sitter         | Language-agnostic AST parsing for code scanner (Rust, Go, TS, Java, Python, C#) |
| Git analysis         | gix (gitoxide)      | Pure Rust git implementation — no dependency on `git` binary |
| SVG generation       | Hand-written        | Full control over CSS classes, no DOM dependency        |
| Site generation      | Hand-written HTML templates | Minimal dependency; Tera for templating          |
| Layout — layered     | Custom Sugiyama     | Needed for pipelines and architecture diagrams         |
| Layout — force       | Custom Barnes-Hut   | Landscape views with many nodes                        |
| Layout — gitgraph    | Custom lane-based   | Git-specific: branches as swim lanes, time axis        |
| PNG/PDF export       | resvg (optional)    | Pure Rust SVG rasterizer, no browser needed            |
| MCP server           | tower + rmcp        | JSON-RPC over stdio or HTTP; thin integration layer    |
| Hugo integration     | CLI subprocess      | Hugo calls `forge build` via shortcode exec; no Go code needed |
| MkDocs integration   | CLI subprocess      | MkDocs plugin calls `forge build`; plugin is <50 lines of Python |
| JSON schema          | JSON Schema Draft 2020-12 | For model interchange and AI integration         |
| LSP                  | tower-lsp           | Rust LSP framework, async, well-maintained             |
| Search index         | tantivy (optional)  | Pure Rust full-text search for generated doc sites     |

### Dependencies (Rust crate count target: < 40)

Core: `pest`, `serde`, `serde_json`, `clap`, `notify` (file watching), `tree-sitter` + language grammars, `gix`, `tera` (templating), `tokio` (async for LSP/serve/MCP).

Optional (behind feature flags): `resvg` (PNG/PDF export), `tower-lsp` (LSP), `rmcp` (MCP server), `tantivy` (search index), `gifski` (GIF export).

### Binary Size Target

The default build (without optional features) should produce a binary under 15 MB. With all features enabled, under 30 MB. Static linking via `musl` for portable Linux deployment; universal binary for macOS (arm64 + x86_64).

---

## 12. Comparison with Existing Tools

| Capability                    | Structurizr | Mermaid  | Forge              |
|-------------------------------|-------------|----------|--------------------|
| C4 architecture model         | Yes         | Partial  | Yes                |
| Single semantic model         | Yes         | No       | Yes                |
| Git branching diagrams        | No          | Yes      | Yes                |
| CI/CD pipeline diagrams       | No          | No       | Yes                |
| Process modeling              | No          | No       | Yes                |
| Multi-file composition        | `!include`  | No       | `!include`, `!extends`, `!fragment`, `!if` |
| Animated diagrams             | No          | No       | CSS/SMIL/GIF/WebM  |
| SVG with CSS classes          | No          | No       | Yes                |
| Codebase scanning             | Partial     | No       | `forge analyze` — code, git, CI, k8s, docker, openapi |
| Static doc site generation    | Web app     | No       | `forge generate` — full static site with search |
| Architectural diff            | No          | No       | `forge generate --diff` / `forge diff` |
| Architectural linting         | No          | No       | `forge check` — built-in + custom rules, SARIF output |
| MCP server for AI agents      | No          | No       | `forge mcp` — full tool suite for AI integration |
| AI model generation           | No          | No       | Via MCP + analyze  |
| Static-site integration       | Limited     | Yes      | Hugo, MkDocs, Docusaurus |
| Deployment                    | JVM + web   | Node/CDN | Single Rust binary |
| Custom themes via CSS         | Partial     | Partial  | First-class        |
| Outline/wireframe mode        | No          | No       | `--style outline`  |

---

## 13. Example: Full Model File

A complete example showing all domains working together:

```forge
forge "Acme E-Commerce" {
  description "Architecture and delivery for Acme's online store"

  model {
    shopper = person "Shopper" {
      description "Browses and purchases products"
    }

    store = system "Online Store" {
      web = container "Web Frontend" {
        technology "React / Next.js"
      }
      catalog = container "Catalog Service" {
        technology "Rust / Axum"
      }
      orders = container "Order Service" {
        technology "Rust / Axum"
      }
      db = container "Product DB" {
        technology "PostgreSQL"
        tags "database"
      }

      web -> catalog "fetches products" "GraphQL"
      web -> orders "places orders" "gRPC"
      catalog -> db "reads" "SQL"
      orders -> db "reads/writes" "SQL"
    }

    shopper -> store.web "browses and buys" "HTTPS"
  }

  process {
    repo = repository "acme-store" {
      system store
    }

    strategy "github-flow" {
      main = branch "main" {
        protection "require-review" "require-ci" "no-force-push"
      }
      feature = branch "feature/*" {
        branchesFrom main
        mergesInto main
      }
    }

    pipeline "store-ci" {
      triggers repo.main on "push"

      test = stage "Test" {
        step "cargo test --workspace"
        step "npm test --prefix web"
      }

      build = stage "Build Images" {
        needs test
        step "docker build -t catalog:$SHA ."
        step "docker build -t orders:$SHA ."
      }

      deploy = stage "Deploy" {
        needs build
        environment production
        gate "smoke-tests-pass"
      }
    }
  }

  flow "typical-sprint" {
    branch main
    commit id:"m1" "Sprint start"

    branch "feature/search" from main
    commit id:"f1" "Add search index"
    commit id:"f2" "Search UI"

    checkout main
    branch "feature/checkout-fix" from main
    commit id:"c1" "Fix cart total"
    merge main id:"c-merge"

    checkout "feature/search"
    commit id:"f3" "Tests"
    merge main id:"f-merge"

    commit id:"m2" tag:"v2.3.0" "Release 2.3"
  }

  views {
    systemContext store "Context" {
      include *
      autoLayout lr
    }

    container store "Containers" {
      include *
      autoLayout tb
    }

    gitGraph "typical-sprint" "SprintFlow" {
      orientation lr
      showTags true
    }

    pipelineView "store-ci" "Pipeline" {
      include *
      autoLayout lr
    }
  }

  styles {
    element "person"           { shape person; background "#08427B"; color "#fff" }
    element "container"        { shape roundedBox; background "#438DD5"; color "#fff" }
    element[tag="database"]    { shape cylinder; background "#1168BD" }
    relationship *             { color "#707070"; style dashed }
    branch "main"              { color "#2E7D32"; lineWidth 3 }
    branch "feature/*"         { color "#1565C0" }
    stage *                    { background "#F5F5F5"; border "#BDBDBD" }
    gate *                     { shape diamond; background "#FFF3E0" }
  }
}
```

---

## 14. Resolved Design Decisions

1. **Workspace composition** — Resolved: Forge uses `!include` directives with relative path resolution, glob patterns, remote URLs, and circular-include detection. Named `!fragment` / `!use` blocks provide reusable patterns, and `!extends` / `!override` support workspace inheritance. See §3.6.
2. **Animation** — Resolved: Animation is a first-class view property using ordered frames. Output uses CSS keyframes by default (self-contained SVG, no JS), with SMIL, GIF, and WebM as alternative render modes. A minimal optional script (< 1KB) enables interactive playback in browsers. See §3.7.
3. **Single binary** — Resolved: Everything ships as one Rust binary. No Go helper, no Python runtime. Hugo/MkDocs integration is via CLI subprocess calls (`forge build`), not native plugins. This eliminates deployment complexity and version-skew issues.
4. **Codebase analysis** — Resolved: `forge analyze` uses tree-sitter for language-agnostic AST parsing and gix for pure-Rust git access. Scanners are modular but compile into the single binary. AI augmentation happens via MCP, not a built-in LLM client.
5. **Architectural linting** — Resolved: `forge check` uses a declarative rule engine. Built-in rules cover common architectural anti-patterns. Custom rules use a `.forge-rules` DSL. Output supports SARIF for CI integration.
6. **Doc site generation** — Resolved: `forge generate` produces a complete static site using built-in Tera templates. No external static site generator dependency. The site includes per-element detail pages, search, and optional diff overlays.
7. **Model diffing** — Resolved: `forge diff` compares two model snapshots (files or git revisions) and produces a typed changeset. The diff can be rendered as SVG overlays (CSS classes for added/removed/modified) or as a standalone report page in the generated site.
8. **MCP server** — Resolved: `forge mcp` exposes the full toolchain over MCP (stdio or HTTP). AI agents can query models, render views, run checks, analyze code, and compute diffs. Write operations (analyze, fix) are opt-in via `--allow-write`.

## 15. Open Questions

1. **WASM target** — Should the Rust core compile to WASM for browser-based rendering (e.g., live preview in VS Code webview, playground site)?
2. **Custom element kinds** — Plugin API for domain-specific elements (e.g., `dataFlow`, `threatModel`)? What's the extension mechanism given the single-binary constraint? (Possible approach: WASM-based plugins loaded at runtime.)
3. **Fragment parameterisation** — Should `!fragment` support formal parameters (like function arguments), or rely on convention-based `$variable` substitution from the enclosing scope?
4. **Animation interactivity** — Should the optional playback script support richer interaction (e.g., click-to-zoom on a frame, tooltip overlays), or stay minimal to preserve the "no JS required" principle?
5. **Incremental analysis** — Should `forge analyze` cache previous scan results and only re-analyze changed files? What's the invalidation strategy for cross-file dependency inference?
6. **Rule severity escalation** — Should `forge check` support ratcheting (once a rule passes for all elements, new violations become errors even if the rule is normally a warning)?
7. **MCP authentication** — For the HTTP transport, what authentication mechanism? API keys, mTLS, or rely on the host environment (e.g., SSH tunnel)?

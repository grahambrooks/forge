# Forge — A Unified Software Modeling DSL

## 1. Vision

Forge is a text-based modeling language and toolchain for describing the *structure* of software systems and the *processes* used to build and deliver them — from a single, coherent model. Where Structurizr focuses on architecture (C4 containers, components, deployment) and Mermaid provides ad-hoc diagram types, Forge unifies both under one semantic model so that a system's branching strategy, CI/CD pipeline, architecture, and deployment topology all reference the same elements and can be rendered as multiple views.

### Design Principles

1. **One model, many views.** Every diagram is a projection of a shared semantic graph — never a standalone drawing.
2. **Process is a first-class citizen.** Git-flow, trunk-based development, release trains, CI/CD pipelines, and incident runbooks live alongside containers and components.
3. **SVG-native, CSS-styled.** All output is clean SVG with well-named CSS classes, so users can theme diagrams with their own stylesheets.
4. **Embeddable everywhere.** Output works inline in Markdown, Hugo, MkDocs, Docusaurus, and any static-site generator.
5. **Fast and dependency-light.** Core engine in Rust; optional Go helper for site-generator plugins. Zero runtime JavaScript required for rendered output.
6. **AI-augmentable.** The model format is designed to be both human-writable and machine-generatable from source code, git history, and CI config.

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

### 4.1 Component Overview

```
┌─────────────────────────────────────────────────┐
│                   forge (CLI)                    │
│                                                  │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
│  │  Parser   │  │  Model   │  │   Renderer    │  │
│  │ (pest/   │─▶│  Graph   │─▶│  (SVG writer) │  │
│  │  nom)    │  │          │  │               │  │
│  └──────────┘  └──────────┘  └───────────────┘  │
│                      │              │            │
│                      ▼              ▼            │
│               ┌──────────┐  ┌───────────────┐   │
│               │ Validator │  │ Layout Engine │   │
│               └──────────┘  └───────────────┘   │
│                                     │            │
│                              ┌──────────────┐   │
│                              │  CSS Theming  │   │
│                              └──────────────┘   │
└─────────────────────────────────────────────────┘
        │              │              │
        ▼              ▼              ▼
   .forge files    model.json     *.svg output
                   (intermediate)
```

### 4.2 Module Breakdown

| Module            | Language | Responsibility                                                  |
|-------------------|----------|-----------------------------------------------------------------|
| `forge-parser`    | Rust     | PEG/packrat parser (pest) for `.forge` files → AST; handles `!include`, `!extends`, `!fragment`, `!use`, `!if`, globs, and cycle detection |
| `forge-model`     | Rust     | Semantic graph: nodes, edges, properties, validation, queries   |
| `forge-layout`    | Rust     | Auto-layout algorithms — layered (Sugiyama), force-directed, grid |
| `forge-render`    | Rust     | Model → SVG with CSS class annotations; animation frame generation (CSS keyframes, SMIL, GIF/WebM export) |
| `forge-cli`       | Rust     | CLI entry point: `forge build`, `forge watch`, `forge export`   |
| `forge-lsp`       | Rust     | Language Server Protocol for editor integration                 |
| `forge-ai`        | Rust/Python | AI bridge: scan source code, git log, CI config → `.forge`   |
| `forge-plugin-hugo`   | Go   | Hugo shortcode / render-hook plugin                            |
| `forge-plugin-mkdocs` | Python | MkDocs plugin (thin wrapper calling forge CLI)              |

### 4.3 Data Flow

```
 Source Code ──┐
 Git History ──┤    ┌──────────┐    ┌───────────┐    ┌──────────┐
 CI Config  ───┼──▶ │ forge-ai │──▶ │  .forge   │──▶ │  Parser  │
 Manual Edit ──┘    └──────────┘    │  files    │    └────┬─────┘
                                    └───────────┘         │
                                                          ▼
                                                   ┌──────────┐
                                         ┌────────▶│  Model   │◀── validate
                                         │         │  Graph   │
                                         │         └────┬─────┘
                                         │              │
                                    ┌────┴────┐   ┌─────▼──────┐
                                    │  JSON   │   │   Views    │
                                    │ export  │   │ (filtered  │
                                    └─────────┘   │  subgraphs)│
                                                  └─────┬──────┘
                                                        │
                                                  ┌─────▼──────┐
                                                  │   Layout   │
                                                  │   Engine   │
                                                  └─────┬──────┘
                                                        │
                                                  ┌─────▼──────┐
                                                  │  SVG + CSS │──▶ embed in
                                                  │  Render    │    Markdown
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

## 7. AI Model Generation

### 7.1 Source Scanners

Forge ships with pluggable scanners that produce `.forge` fragments:

| Scanner              | Input                        | Output                          |
|----------------------|------------------------------|---------------------------------|
| `forge scan code`    | Source tree                  | Components, dependencies        |
| `forge scan git`     | Git log + branch refs        | Flow definitions, branch strategy |
| `forge scan ci`      | GitHub Actions / GitLab CI   | Pipeline and stage definitions  |
| `forge scan k8s`     | Kubernetes manifests         | Deployment nodes, artifacts     |
| `forge scan docker`  | Dockerfiles, Compose files   | Containers, networking          |
| `forge scan openapi` | OpenAPI specs                | API containers, relationships   |

### 7.2 AI Augmentation

```bash
# Use an LLM to infer high-level system descriptions from code
forge ai describe --source ./src --model claude

# Merge AI-generated fragments into an existing model
forge ai merge --base architecture.forge --fragments ai-output/

# Interactive: AI proposes, human reviews
forge ai suggest --watch
```

The AI layer operates on the intermediate JSON representation, so any LLM with structured output can participate. The `forge-ai` module provides:

- Prompt templates tuned for architecture extraction
- A diff/merge algorithm for `.forge` files
- Confidence scores on inferred elements (rendered as dashed lines in views until confirmed)

---

## 8. CLI Interface

```
forge — A unified software modeling tool

USAGE:
    forge <COMMAND> [OPTIONS]

COMMANDS:
    build       Parse .forge files and render views to SVG
    watch       Watch for changes and rebuild incrementally
    validate    Check model consistency and completeness
    export      Export model as JSON, YAML, or Structurizr DSL
    import      Import from Structurizr DSL, PlantUML, or Mermaid
    scan        Auto-detect architecture from source artifacts
    ai          AI-assisted model generation and refinement
    serve       Start a local preview server with live reload
    lsp         Start the Language Server Protocol server

BUILD OPTIONS:
    --source <FILE>        Input .forge file (default: ./forge.forge)
    --view <NAME>          Render a specific view (default: all)
    --format <FORMAT>      Output format: svg, svg-inline, png, pdf
    --theme <FILE>         CSS theme file
    --out <DIR>            Output directory (default: ./forge-output/)
    --jobs <N>             Parallel rendering jobs (default: CPU count)

EXAMPLES:
    forge build
    forge build --view SystemContext --format svg
    forge watch --serve
    forge scan code --source ./src --output arch-fragments/
    forge ai describe --source ./src
    forge export --format structurizr
    forge import --from structurizr --input workspace.dsl
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

- `forge-parser`: PEG grammar for the full DSL using `pest`
- **File composition**: `!include` (path + glob), `!fragment` / `!use`, circular-include detection
- `forge-model`: In-memory graph with validation
- `forge-render`: SVG output for structure views (systemContext, container, component)
- `forge-cli`: `build`, `validate`, `export` commands
- Basic auto-layout (layered / Sugiyama)

### Phase 2 — Process & Flow (Months 3–5)

- Process domain: pipeline, stage, gate, environment
- Flow domain: gitgraph rendering (branch, commit, merge, tag)
- `pipelineView` and `gitGraph` view types
- **Animation engine**: frame-based animation with CSS keyframes output
- CSS theming system
- `forge watch` with incremental rebuild
- `!extends` / `!override` for workspace inheritance
- `!include <url>` for remote includes with content-addressable caching

### Phase 3 — Integration (Months 5–7)

- Hugo and MkDocs plugins (with animation playback script injection)
- `forge serve` with live reload and `--present` mode for animated walkthroughs
- Import/export: Structurizr DSL, PlantUML C4, Mermaid
- Deployment views
- `forge-lsp` for VS Code / Neovim
- SMIL animation output mode
- `!if` conditional includes

### Phase 4 — AI & Scanning (Months 7–10)

- Source scanners: code, git, CI, Kubernetes, Docker, OpenAPI
- AI model generation with confidence scoring
- Interactive `forge ai suggest` mode
- Composite and dynamic views

### Phase 5 — Polish (Months 10–12)

- PNG/PDF export via `resvg`
- Animated GIF and WebM export (via `resvg` + `gifski`)
- Force-directed layout for landscape views
- Performance optimization (target: 10k-element models in < 1s)
- Plugin API for custom element kinds and renderers
- Documentation and example gallery

---

## 11. Technology Choices

| Concern              | Choice              | Rationale                                              |
|----------------------|---------------------|--------------------------------------------------------|
| Core language        | Rust                | Performance, safety, single binary, no runtime         |
| Parser               | pest (PEG)          | Fast, readable grammar files, good error messages      |
| SVG generation       | Hand-written        | Full control over CSS classes, no DOM dependency        |
| Layout — layered     | Custom Sugiyama     | Needed for pipelines and architecture diagrams         |
| Layout — force       | Custom Barnes-Hut   | Landscape views with many nodes                        |
| Layout — gitgraph    | Custom lane-based   | Git-specific: branches as swim lanes, time axis        |
| PNG/PDF export       | resvg               | Pure Rust SVG rasterizer, no browser needed            |
| Hugo plugin          | Go                  | Native Hugo module, compiled into Hugo's binary        |
| MkDocs plugin        | Python (thin shim)  | Calls `forge` CLI subprocess                           |
| JSON schema          | JSON Schema Draft 2020-12 | For model interchange and AI integration         |
| LSP                  | tower-lsp           | Rust LSP framework, async, well-maintained             |

### Dependencies (Rust crate count target: < 30)

Core: `pest`, `serde`, `serde_json`, `clap`, `notify` (file watching), `resvg` (optional), `tower-lsp` (optional), `tokio` (async for LSP/serve only).

---

## 12. Comparison with Existing Tools

| Capability                | Structurizr | Mermaid  | Forge        |
|---------------------------|-------------|----------|--------------|
| C4 architecture model     | Yes         | Partial  | Yes          |
| Single semantic model     | Yes         | No       | Yes          |
| Git branching diagrams    | No          | Yes      | Yes          |
| CI/CD pipeline diagrams   | No          | No       | Yes          |
| Process modeling           | No          | No       | Yes          |
| Multi-file composition    | `!include`  | No       | `!include`, `!extends`, `!fragment`, `!if` |
| Animated diagrams         | No          | No       | CSS/SMIL/GIF/WebM |
| SVG with CSS classes      | No          | No       | Yes          |
| Static-site integration   | Limited     | Yes      | Yes          |
| AI model generation       | No          | No       | Yes          |
| Source code scanning      | Partial     | No       | Yes          |
| Rust/Go performance       | Java        | JS       | Rust + Go    |
| Minimal dependencies      | JVM         | Node     | Single binary|
| Custom themes via CSS     | Partial     | Partial  | First-class  |

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

## 15. Open Questions

1. **Bidirectional sync** — When AI generates model fragments, how much should be auto-merged vs. human-reviewed? What's the conflict resolution strategy?
2. **WASM target** — Should the Rust core compile to WASM for browser-based rendering (e.g., live preview in VS Code webview)?
3. **Custom element kinds** — Plugin API for domain-specific elements (e.g., `dataFlow`, `threatModel`)? What's the extension mechanism?
4. **Fragment parameterisation** — Should `!fragment` support formal parameters (like function arguments), or rely on convention-based `$variable` substitution from the enclosing scope?
5. **Animation interactivity** — Should the optional playback script support richer interaction (e.g., click-to-zoom on a frame, tooltip overlays), or stay minimal to preserve the "no JS required" principle?

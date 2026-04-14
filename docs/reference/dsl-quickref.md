# DSL quick reference

A one-page cheat sheet for the `.forge` syntax. For a complete
end-to-end example that uses every feature, see
[`forge/examples/payments.forge`](../../forge/examples/payments.forge).
For the underlying design, see [`DESIGN.md`](../../DESIGN.md).

## File structure

Every `.forge` file starts with a top-level `forge` block. Everything
else nests inside it.

```forge
forge "Name" {
  description "Optional summary"

  model { ... }
  process { ... }
  deployment "env" { ... }
  techStack { ... }
  dataModel { ... }
  trustBoundaries { ... }
  teams { ... }
  apis { ... }
  eventFlows { ... }
  envConfig { ... }
  slos { ... }
  dependencies { ... }
  docs { ... }
  views { ... }
}
```

Only `model` is strictly required. Everything else is optional.

## Declaring elements

Every element uses `id = kind "Name" { … }`. The id is how you
reference it elsewhere.

```forge
id = kind "Display Name" {
  description "One-line summary"
  technology "Rust / Axum"
  tags "tag1" "tag2"
}
```

Kinds you can declare at the top of the `model` block:

| Kind | Meaning |
| --- | --- |
| `person` | An external human actor |
| `system` | A software system (can contain containers) |
| `container` | A runnable/deployable unit inside a system |
| `component` | A code-level module inside a container |

Systems can nest containers inline; containers can nest components
inline. Dotted notation (`payments.api`) references the nested element.

## Relationships

```forge
source -> target "label"
source -> target "label" "technology"
```

Both forms work at any nesting level. The third string is optional
technology metadata (e.g. `"HTTPS"`, `"gRPC"`, `"SQL"`).

## `model` block example

```forge
model {
  customer = person "Customer"

  payments = system "Payment Service" {
    description "Handles card and bank payments"
    tags "core" "pci"

    api = container "Payment API" {
      technology "Rust / Actix"

      rest = component "REST Controller" {
        technology "Actix-web"
      }
    }

    db = container "Ledger DB" {
      technology "PostgreSQL 16"
      tags "database"
    }

    api -> db "reads/writes" "SQL"
  }

  customer -> payments.api "makes payments" "HTTPS"
}
```

## `process` block

CI/CD pipeline, branching strategy, and repository.

```forge
process {
  repo = repository "payments-api" {
    url "https://github.com/acme/payments-api"
    system payments
  }

  strategy "trunk-based" {
    trunk = branch "main" {
      protection "require-review" "require-ci"
    }
    feature = branch "feature/*" {
      branchesFrom trunk
      mergesInto trunk
    }
  }

  pipeline "payments-ci" {
    triggers repo.main on "push"

    build = stage "Build & Test" {
      step "cargo build --release"
      step "cargo test"
    }

    deploy = stage "Deploy" {
      needs build
      environment production
      gate "manual-approval" {
        approvers "platform-team"
      }
    }
  }
}
```

## `deployment` block

Deployment topology: nested nodes, each with a technology and
(optionally) container instances.

```forge
deployment "production" {
  node "AWS" {
    technology "Amazon Web Services"
    node "us-east-1" {
      technology "AWS Region"
      node "EKS Cluster" {
        technology "Kubernetes 1.29"
        node "API Pods" {
          technology "3 replicas"
          instance api
        }
      }
    }
  }
}
```

`instance <id>` binds a deployment node to a container declared in
the `model` block.

## `techStack` block

Categorised inventory of technologies used in the system.

```forge
techStack {
  category "Languages & Frameworks" {
    tech "Rust" { version "1.75" purpose "Payment API and Processor" }
    tech "Actix-web" { version "4" purpose "HTTP/REST framework" }
  }
  category "Data Stores" {
    tech "PostgreSQL" { version "16" purpose "Ledger data" }
  }
}
```

## `dataModel` block

Entity-relationship model with field-level detail.

```forge
dataModel {
  entity "Transaction" {
    field "id" "UUID" "PK"
    field "amount" "DECIMAL(19,4)" "NOT NULL"
    field "customer_id" "UUID" "FK"
    owner db
  }

  entity "Customer" {
    field "id" "UUID" "PK"
    field "email" "VARCHAR(255)" "UNIQUE"
    owner db
  }

  relationship "Customer" "Transaction" {
    label "places"
    cardinality "1:N"
  }
}
```

`owner <id>` binds the entity to the container that stores it.

## `trustBoundaries` block

Security zones with element membership. Zone levels: `public`, `dmz`,
`internal`, `pci`.

```forge
trustBoundaries {
  boundary "Public Internet" {
    level "public"
    includes customer
  }
  boundary "PCI Data Zone" {
    level "pci"
    includes payments.db
    includes payments.cache
  }
}
```

## `teams` block

Team ownership over containers. Populated automatically from
`.github/CODEOWNERS` when `forge analyze` runs; can also be written
by hand.

```forge
teams {
  team "Platform Team" {
    owns payments.api
    owns payments.processor
    contact "#platform-eng on Slack"
  }
}
```

## `apis` block

Endpoint catalog per container.

```forge
apis {
  api payments.api {
    endpoint "POST /payments" {
      description "Create a new payment"
      request "{ amount, currency, customer_id }"
      response "{ id, status, created_at }"
    }
    endpoint "GET /payments/{id}" {
      description "Get payment details"
      response "{ id, amount, currency, status }"
    }
  }
}
```

## `eventFlows` block

Message/event flow model with publishers and subscribers.

```forge
eventFlows {
  flow "payment-completed" {
    topic "payments.events.completed"
    description "Emitted after successful payment capture"
    publisher payments.processor
    subscriber payments.notifier
  }
}
```

## `envConfig` block

Per-environment configuration values.

```forge
envConfig {
  env "staging" {
    PAYMENT_GATEWAY "stripe-test"
    DATABASE_URL "postgres://staging-rds:5432/payments"
  }
  env "production" {
    PAYMENT_GATEWAY "stripe-live"
    DATABASE_URL "postgres://prod-rds:5432/payments"
  }
}
```

## `slos` block

Service-level objectives per container.

```forge
slos {
  slo payments.api {
    latency "< 200ms p99"
    availability "99.99%"
    error_budget "0.01% per month"
  }
}
```

## `dependencies` block

External systems you depend on but don't own.

```forge
dependencies {
  dependency "Stripe" {
    kind "payment-processor"
    criticality "critical"
    url "https://api.stripe.com"
    description "Primary payment gateway"
  }
}
```

Valid `kind` values are free-form; `criticality` is conventionally
one of `critical`, `high`, `medium`, `low`.

## `docs` block

Markdown pages to include in generated sites.

```forge
docs {
  page "overview" "docs/overview.md" { order 1 }
  page "adr-0001" "docs/adr-0001-rust-for-api.md" { order 2 }
}
```

## `views` block {#views}

Named views that pick elements from the model and lay them out. Every
view has a key (the filename it produces) and can optionally set a
title and layout direction.

```forge
views {
  systemContext payments "Context" {
    include *
    autoLayout lr
    title "Payment Platform — System Context"
  }

  container payments "Containers" {
    include *
    autoLayout tb
  }

  component payments.api "APIComponents" {
    include *
  }

  pipelineView "payments-ci" "Pipeline" {
    include *
  }

  deploymentView "production" "Production Topology" {
    include *
  }

  techStackView "TechStack" {
    include *
  }

  branchingView "Branching" {
    include *
  }

  dataModelView "DataModel" {
    include *
  }

  trustBoundaryView "TrustBoundaries" {
    include *
  }

  teamView "TeamMap" {
    include *
  }

  apiCatalogView "APICatalog" {
    include *
  }

  eventFlowView "EventFlows" {
    include *
  }

  dynamic payments "LoginFlow" {
    title "User login sequence"
    1. customer -> payments.api "POST /login" "HTTPS"
    2. payments.api -> payments.db "SELECT user"
    3. payments.api -> customer "JWT + session cookie"
  }

  composite "Dashboard" {
    title "Executive overview"
    grid 2 2
    cellSize 600 400
    cell "SystemContext"
    cell "Containers"
    cell "Pipeline"
    cell "Production Topology"
  }
}
```

`include *` means "every element in scope of this view." You can also
list element ids explicitly: `include payments.api payments.db`.

`autoLayout` takes `lr` (left-to-right) or `tb` (top-to-bottom).

### `dynamic` views

A dynamic view is a container view with **ordered relationships** —
each `<num>. src -> dst "label"` inside the block records the step
number on the relationship. The renderer draws a circled step badge
near each arrow's midpoint, and stepping through the view in
`forge serve --present` mode auto-generates one animation frame per
step. The frame for step N includes every element and relationship
with `order ≤ N`, so viewers watch the flow build up cumulatively.
If you want different animation semantics, add an explicit
`animation { frames … }` block and it will override the derived one.

### `composite` views

Composite views embed other views in a grid. The DSL:

- `grid <cols> <rows>` — the layout. Rows are optional; if you omit
  `rows` forge infers it from the cell count.
- `cellSize <w> <h>` — per-cell pixel dimensions. Default `600 400`.
- `cell "<view-key>"` — adds a cell referencing another view by its
  key, in row-major order.

Each cell carries a thin frame and a small caption showing the
referenced view key. Nested composites (a composite that references
another composite) are short-circuited — the inner reference is
skipped rather than recursed into.

## Animated views

Any view can carry an `animation { … }` block with frame-by-frame
instructions for presentation mode:

```forge
container payments "PaymentFlow" {
  include *
  autoLayout tb
  animation {
    frame "Step 1: Customer submits payment" {
      include customer payments.api
      highlight customer
    }
    frame "Step 2: API validates" {
      include customer payments.api
      highlight payments.api { pulse true }
    }
    frame "Step 3: Processor takes over" {
      include-all
      highlight payments.processor
      state payments.api { color "blue" }
    }
  }
}
```

View this with `forge serve --present`.

## Preprocessor directives

Three directives let you split models across files and toggle sections
by environment:

```forge
// Pull in another file (path or glob)
!include "shared/actors.forge"
!include "services/*.forge"

// Reusable fragment + use
!fragment database_container {
  technology "PostgreSQL 16"
  tags "database"
}

payments_db = container "Payments DB" {
  !use database_container
}

// Conditional block
!if env("FORGE_ENV") == "production" {
  slos {
    slo api { availability "99.99%" }
  }
}
```

## See also

- [`DESIGN.md`](../../DESIGN.md) — the full design spec (richer, more
  speculative)
- [`forge/examples/payments.forge`](../../forge/examples/payments.forge)
  — a complete reference model
- [Linter rules](linter-rules.md) — what the built-in linter expects
  from a well-formed model

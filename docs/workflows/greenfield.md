# Greenfield: starting from scratch

Use this workflow when you're designing a system that doesn't exist
yet — an ADR in progress, a whiteboard you want to preserve, a design
review you're about to walk your team through.

No `forge analyze`. No existing codebase. Just you and a blank file.

## 1. Scaffold the file

```bash
mkdir my-system && cd my-system
```

Create `architecture.forge`:

```forge
forge "My System" {
  description "One-line summary of what this system does"

  model {
    // Actors first — who uses it?
    customer = person "Customer"
    admin    = person "Administrator"

    // Then the system and its containers
    app = system "My System" {
      description "What the system does at a high level"

      web = container "Web UI" {
        technology "TypeScript / Next.js"
        description "Customer-facing surface area"
      }

      api = container "API" {
        technology "Rust / Axum"
        description "HTTP and background jobs"
      }

      db = container "Database" {
        technology "PostgreSQL 16"
        tags "database"
      }
    }

    // Finally, the relationships
    customer -> app.web "uses" "HTTPS"
    app.web  -> app.api "calls"  "HTTPS + JWT"
    app.api  -> app.db  "reads/writes" "SQL"
  }

  views {
    systemContext app "Context" {
      include *
      autoLayout lr
    }
    container app "Containers" {
      include *
      autoLayout tb
    }
  }
}
```

## 2. Preview as you edit

```bash
forge serve --source architecture.forge --port 4000
```

Open <http://localhost:4000>. Every save rerenders.

## 3. Add the process side

Real architecture has delivery context too — branching strategy,
pipeline, deployment topology. Add the `process` and `deployment`
blocks next to `model`:

```forge
  process {
    repo = repository "my-system" {
      url "https://github.com/acme/my-system"
      system app
    }

    strategy "trunk-based" {
      trunk = branch "main"
      feature = branch "feature/*" {
        branchesFrom trunk
        mergesInto trunk
      }
    }

    pipeline "ci" {
      triggers repo.main on "push"

      build = stage "Build & Test" {
        step "cargo build --release"
        step "cargo test"
      }

      deploy-staging = stage "Deploy Staging" {
        needs build
        environment staging
      }

      deploy-prod = stage "Deploy Production" {
        needs deploy-staging
        environment production
        gate "manual-approval"
      }
    }
  }

  deployment "production" {
    node "AWS" {
      technology "Amazon Web Services"
      node "us-east-1" {
        technology "AWS Region"
        node "EKS" {
          technology "Kubernetes 1.29"
          node "api-pods" {
            technology "3 replicas"
            instance api
          }
        }
        node "RDS" {
          technology "Managed PostgreSQL"
          instance db
        }
      }
    }
  }
```

Then add the corresponding views:

```forge
  views {
    // ... existing systemContext / container views ...

    pipelineView "ci" "Pipeline" {
      include *
      title "My System — CI/CD Pipeline"
    }

    deploymentView "production" "Production Topology" {
      include *
      title "My System — Production Topology"
    }
  }
```

Save. The preview server picks up the new views automatically.

## 4. Lint before commit

```bash
forge check --source architecture.forge
```

Address every `warning` and `error` the linter surfaces. The usual
greenfield findings are missing descriptions, missing technology
labels, and orphaned elements you sketched in but haven't wired up
yet. See [Linting](../user-guide/linting.md).

## 5. Commit

```bash
git init
git add architecture.forge
git commit -m "Initial architecture model"
```

At this point the file is the canonical description of your design.
Every change goes through a normal PR review and the `forge check`
step catches regressions.

## 6. When code starts landing

Once the greenfield system starts having real code, switch to the
[brownfield workflow](brownfield.md) to have `forge analyze` check the
implementation against the design. You'll want to run it in
[merge mode](../user-guide/merge-mode.md) so your hand-authored model
survives.

## See also

- [Your first model](../user-guide/first-model.md) — even shorter
  starter
- [DSL quick reference](../reference/dsl-quickref.md) — every block
  type you can write
- [`forge/examples/payments.forge`](../../forge/examples/payments.forge)
  — a full-fat reference model that exercises every feature

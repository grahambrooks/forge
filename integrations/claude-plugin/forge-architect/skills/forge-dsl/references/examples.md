# Forge DSL pattern cookbook

Common shapes. Copy, adapt, validate with `forge_validate` before saving.

## Minimal valid model

```forge
forge "Demo" {
  model {
    user = person "User"
    web  = system "Web App" {
      api = container "API" { technology "Go" }
      db  = container "DB"  { technology "Postgres"; tags "database" }
      api -> db "reads/writes"
    }
    user -> web.api "uses" "HTTPS"
  }

  views {
    system-context-view web "Context"    { include *; auto-layout lr }
    container-view       web "Containers" { include *; auto-layout tb }
  }
}
```

## Polyglot monorepo

Three services in one repo, each a container. One System wrapper.

```forge
forge "Acme" {
  model {
    acme = system "Acme Platform" {
      web  = container "Web"  { technology "Next.js" }
      api  = container "API"  { technology "Go" }
      jobs = container "Jobs" { technology "Python" }
      db   = container "DB"   { technology "PostgreSQL"; tags "database" }

      web  -> api "fetches"            "HTTPS/JSON"
      api  -> db  "reads/writes"       "SQL"
      jobs -> db  "batch updates"      "SQL"
    }
  }

  views {
    container-view acme "Containers" { include *; auto-layout tb }
  }
}
```

## Event-driven system

Event flow with Kafka topic elements, plus dynamic view showing the flow.

```forge
forge "Orders" {
  model {
    orders = system "Orders" {
      checkout = container "Checkout"   { technology "Node" }
      fulfilment = container "Fulfilment" { technology "Go" }
      inventory  = container "Inventory"  { technology "Go" }
      kafka      = container "Events"     { technology "Kafka"; tags "bus" }

      checkout    -> kafka       "publishes order.placed"
      kafka       -> fulfilment  "consumes order.placed"
      fulfilment  -> inventory   "reserves stock"  "gRPC"
      fulfilment  -> kafka       "publishes order.fulfilled"
    }
  }

  views {
    container-view orders "Containers" { include *; auto-layout lr }

    container-view orders "PlaceOrderFlow" {
      title "Place order — step by step"
      auto-layout lr
      animation {
        frame "Customer checks out" {
          include checkout
          include kafka
          include checkout -> kafka
          notes "Checkout publishes order.placed to Kafka."
        }
        frame "Fulfilment reserves stock" {
          include fulfilment
          include inventory
          include fulfilment -> inventory
          include kafka -> fulfilment
        }
        frame "Order fulfilled" {
          include *
          highlight fulfilment -> kafka { color "#1B5E20"; label "Published" }
        }
      }
    }
  }
}
```

## Kubernetes deployment binding

Binding model containers to pods in a cluster.

```forge
forge "Acme" {
  model {
    acme = system "Acme" {
      api = container "API" { technology "Go" }
      db  = container "DB"  { technology "Postgres"; tags "database" }
      api -> db "reads/writes"
    }
  }

  deployment production "Production" {
    node aws "AWS" {
      node us-east-1 "us-east-1" {
        node eks "EKS" {
          technology "Kubernetes 1.29"
          node api-pods "API Pods" {
            technology "3 replicas"
            instance api
          }
        }
        node rds "RDS" {
          technology "Managed Postgres"
          instance db
        }
      }
    }
  }

  views {
    container-view   acme       "Containers" { include *; auto-layout tb }
    deployment-view  production "Prod"       { include *; auto-layout tb }
  }
}
```

## Data classification + trust boundaries

Flag PII-bearing containers and group them into a PCI zone; the `data-class-boundary` rule then runs clean.

```forge
forge "Payments" {
  model {
    p = system "Payments" {
      api = container "API" { technology "Rust" }
      db  = container "Ledger" {
        technology "Postgres"
        tags "database"
        data-class "pii" "financial"
      }
      api -> db "reads/writes"
    }
  }

  trust-boundaries {
    boundary "Payments PCI Zone" pci {
      member p.api
      member p.db
    }
  }

  views {
    container-view        p "Containers"       { include *; auto-layout tb }
    trust-boundary-view   "TrustBoundaries"    { include * }
  }
}
```

## Multi-file split

`acme.forge` (root):

```forge
forge "Acme" {
  description "Split across multiple files"

  !include "systems/platform.forge"
  !include "systems/billing.forge"

  views {
    !include "views/*.forge"
  }
}
```

`systems/platform.forge`:

```forge
model {
  platform = system "Platform" {
    web = container "Web" { technology "Next.js" }
  }
}
```

The root file is still the only one parsed directly; `!include` is a textual preprocessor that resolves relative to the including file.

## Reusable fragment

```forge
!fragment common-platform-tags {
  tags "platform" "internal"
}

forge "Acme" {
  model {
    p = system "Platform" {
      api = container "API" {
        !use common-platform-tags
        technology "Rust"
      }
    }
  }
}
```

Use sparingly — readers expect to see element attributes inline.

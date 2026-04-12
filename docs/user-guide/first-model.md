# Your first model

This walkthrough writes a small `.forge` file by hand, renders it, and then
previews it in a browser. It's the shortest path from "I just installed
forge" to "I can see my architecture on screen."

If you'd rather have forge build a model from an existing codebase instead
of writing one, jump to [Analyzing a codebase](analyzing-a-codebase.md).

## Create a working directory

```bash
mkdir forge-hello && cd forge-hello
```

## Write a minimal model

Create `architecture.forge` with this content:

```forge
forge "Hello" {
  description "A tiny system to learn the forge DSL"

  model {
    customer = person "Customer" {
      description "Someone who wants to do a thing"
    }

    app = system "Hello App" {
      description "The thing customers want to use"

      web = container "Web UI" {
        technology "TypeScript / Next.js"
        description "Customer-facing website"
      }

      api = container "API" {
        technology "Rust / Axum"
        description "HTTP and gRPC gateway"
      }

      db = container "Database" {
        technology "PostgreSQL 16"
        tags "database"
      }

      web -> api "calls" "HTTPS"
      api -> db "reads/writes" "SQL"
    }

    customer -> app.web "uses" "HTTPS"
  }

  views {
    systemContext app "Context" {
      include *
      autoLayout lr
      title "Hello App — System Context"
    }

    container app "Containers" {
      include *
      autoLayout tb
      title "Hello App — Containers"
    }
  }
}
```

Four things worth noticing:

1. **Top-level `forge "Name" { … }`** is required. Everything else nests
   inside it.
2. **`id = kind "Name" { … }`** is how you declare elements. The left-hand
   side is the id you reference elsewhere (`app.api`); the quoted name is
   the display label.
3. **Relationships use arrow syntax** (`web -> api "calls" "HTTPS"`).
   The third string is optional technology metadata.
4. **Views are separate from the model.** A single model can have many
   views. Each view picks what to include (`include *` = everything in
   scope) and lays it out.

## Render the diagrams

```bash
forge build --source architecture.forge --out _site/diagrams
```

This produces one SVG per view:

```
_site/diagrams/
├── context.svg
└── containers.svg
```

Open either file directly in a browser, or move on to live preview.

## Live preview

```bash
forge serve --source architecture.forge --port 4000
```

Open `http://localhost:4000` in a browser. Every time you save
`architecture.forge`, the server re-renders and the browser reloads
automatically.

Try editing the `api` container's technology to `Rust / Actix` and watch
the diagram update.

## Lint the model

```bash
forge check --source architecture.forge
```

You should see zero violations. If you deliberately delete a description
or technology and re-run, forge will surface `missing-descriptions` or
`missing-technology` warnings.

See [Linter rules](../reference/linter-rules.md) for the full list of
built-in checks.

## Generate a static site

```bash
forge generate --source architecture.forge --out _site
open _site/index.html
```

This produces a self-contained HTML/CSS/SVG site you can host anywhere —
GitHub Pages, S3, your internal static host. See
[Generating docs](generating-docs.md) for the details.

## Where to next

- [DSL quick reference](../reference/dsl-quickref.md) — every block the
  parser understands
- [Linting](linting.md) — add custom `.forge-rules` for team-specific
  architectural constraints
- [Analyzing a codebase](analyzing-a-codebase.md) — let `forge analyze`
  populate most of this file for you, then refine by hand

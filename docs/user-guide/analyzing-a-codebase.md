# Analyzing a codebase

`forge analyze` walks an existing repository, runs every scanner, and
writes a `.forge` file. It's the fastest way to bootstrap a model for a
codebase you already have — instead of starting from a blank page, you
start with whatever the scanners can discover and edit from there.

## The one-liner

```bash
cd path/to/your/repo
forge analyze --out architecture.forge
```

You'll see output like:

```
Scanning...
  Elements: 14
  Relationships: 6
  Wrote: architecture.forge
Done.
```

Open `architecture.forge` in an editor. Every element the scanners
produced is tagged `inferred` (and `inferred:<scanner>`, e.g.
`inferred:code`, `inferred:semantic`, `inferred:correlate`) so you can
tell analyzer output apart from anything you add by hand.

## What each scanner looks for

| Scanner | Reads | Produces |
| --- | --- | --- |
| `code` | `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `requirements.txt`, `Gemfile`, `composer.json`, `pom.xml`, `build.gradle(.kts)`, `build.sbt` | One `container` per manifest, with `technology` inferred from the dep set. Cargo workspaces and npm/pnpm/yarn workspaces expand to one container per member. |
| `semantic` | Source files in Rust, TS, JS, Python, Go, Java, Kotlin, C, C++, C#, Scala, Groovy, Ruby, PHP, Swift | HTTP route endpoints, import relationships, database/queue literals (`postgres://`, `redis://`, `kafka`, …), env-var reads (`process.env.X`, `os.getenv`, `std::env::var`, etc.) |
| `ci` | `.github/workflows/*.yml` | `pipeline` and `stage` elements with `needs:` dependencies, `environment:` declarations, and protection-gate inference |
| `docker` | `Dockerfile`, `Dockerfile.*`, `docker-compose.yml` | One `container` per service; `technology` from `FROM` image; `env_provides` from the `environment:` block; `depends_on` as relationships |
| `git` | `.github/CODEOWNERS`, `CODEOWNERS`, `docs/CODEOWNERS`, local git history | Branching-strategy inference (git-flow, trunk-based, github-flow); `team` elements from CODEOWNERS rules |
| `k8s` | `*.yaml` / `*.yml` with `kind: Deployment`/`StatefulSet`/`DaemonSet`/`Service`/`Ingress`/`ConfigMap` | `DeploymentNode` elements with namespace, replicas, image, and env-var `env_provides`; API endpoints from Ingress rules; `envConfig` from ConfigMaps |
| `infra` | CloudFormation templates, Terraform `.tf`, OpenAPI specs | AWS / GCP containers and deployment nodes; API endpoints from OpenAPI paths |

For the exhaustive list see [Scanners reference](../reference/scanners.md).

## Picking scanners

By default all seven run. You can narrow them with `--scanners` when you
only want part of the picture:

```bash
# Manifest discovery only — fastest, usually what you want first
forge analyze --scanners code --out architecture.forge

# Add CI and docker-compose context without pulling in k8s / terraform
forge analyze --scanners code,semantic,ci,docker --out architecture.forge

# Skip the source-level semantic scan (the slow one on big repos)
forge analyze --scanners code,ci,docker,git,k8s,infra --out architecture.forge
```

The order is significant: `code` always runs before `semantic` so source
files get attributed to the right container, and all scanners run before
the cross-scanner `correlate` pass.

## Excluding directories

`analyze` skips `node_modules`, `target`, `.git`, `vendor`, `dist`, and
`__pycache__` by default. To skip more, pass `--exclude` one or more
times:

```bash
forge analyze --exclude generated --exclude fixtures --out architecture.forge
```

## Dry run

Print to stdout instead of writing a file — handy for peeking at what
the scanners would produce before committing to overwriting anything:

```bash
forge analyze --dry-run | head -50
```

## What you'll see in the output

A `.forge` file produced by analyze looks like this (trimmed):

```forge
forge "my-repo" {
  model {
    api = container "api" {
      description "Rust crate at services/api"
      technology "Rust / Axum"
      tags "inferred" "inferred:code"
    }

    worker = container "worker" {
      description "Rust crate at services/worker"
      technology "Rust"
      tags "inferred" "inferred:code"
    }

    postgres = container "postgres" {
      technology "PostgreSQL"
      tags "inferred" "docker" "database"
    }

    api -> postgres "uses (DATABASE_URL)"
    api -> worker "imports"
  }
}
```

The `inferred` tags are important — they're what makes
[merge mode](merge-mode.md) safe.

## Cross-scanner correlations

After every scanner runs, a post-pass called `correlate` connects facts
that individual scanners couldn't on their own:

- **Env var consumers → providers.** A service that reads `DATABASE_URL`
  in source gets linked to the docker-compose or k8s service that
  declares `DATABASE_URL` in its environment.
- **Connection-string fallback.** When a reader declares a well-known
  URL env var (`DATABASE_URL`, `REDIS_URL`, `MONGO_URL`, `KAFKA_BROKERS`,
  `AMQP_URL`, …) but no exact match exists, forge links it to the
  nearest container tagged `database` or `messaging` whose technology
  matches. This handles the realistic case where postgres only declares
  `POSTGRES_PASSWORD` and not `DATABASE_URL`.
- **CI stages → environments.** Pipeline stages with `environment: prod`
  get a synthetic `Environment` element per unique name, and those
  environments are linked to any k8s DeploymentNodes in a matching
  namespace so you get the full `stage → env → deployment` chain.

See [Correlations reference](../reference/correlations.md) for the
exact rules.

## Next steps

- Your fresh `architecture.forge` is disposable right now — you can
  regenerate it anytime. As soon as you start adding hand-written
  descriptions or relationships, switch to [merge mode](merge-mode.md) so
  re-runs stop trampling your edits.
- Run [`forge check`](linting.md) to surface missing descriptions,
  dependency cycles, and database-direct-access violations.
- Run [`forge serve`](live-preview.md) to see what the scanners
  discovered as diagrams.

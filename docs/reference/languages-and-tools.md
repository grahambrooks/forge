# Languages and tools

What forge knows how to read today. This list reflects the actual
implementation in `main`; if you need something not in here, open
an issue (or a PR — each row maps to a small, contained module).

For the planned roadmap (things the [`DESIGN.md`](../../DESIGN.md)
spec talks about but that aren't implemented yet), see the bottom of
this page.

## Programming languages

Tree-sitter-based semantic analysis is provided by the `symgraph`
crate. The languages below are parsed into symbol and import graphs;
forge then layers framework-specific regex extractors on top for
route detection.

| Language | Manifest parsing (`code`) | Semantic parse (`semantic`) | Route extraction |
| --- | --- | --- | --- |
| Rust | `Cargo.toml` (incl. workspaces) | Yes | Actix `#[get(…)]`, Axum `.route(…, get(h))` |
| TypeScript | `package.json` (+ `tsconfig.json` signal) | Yes | Express `.get(…)`, NestJS `@Get(…)` |
| JavaScript | `package.json` | Yes | Express `.get(…)`, NestJS `@Get(…)` |
| TSX / JSX | `package.json` | Yes | (via TS/JS extractors) |
| Python | `pyproject.toml`, `requirements.txt` | Yes | Flask `@app.route`, FastAPI `@app.get(…)`, Django `path(…)` |
| Go | `go.mod` | Yes | Gin/Echo `.GET(…)`, `http.HandleFunc(…)` |
| Java | `pom.xml`, `build.gradle`, `build.gradle.kts` | Yes | Spring `@GetMapping(…)`, JAX-RS `@Path(…)` |
| Kotlin | `build.gradle.kts` | Yes | Spring `@GetMapping(…)`, JAX-RS |
| C / C++ | — | Yes | — |
| C# | — | Yes | — |
| Scala | `build.sbt` | Yes | — |
| Groovy | `build.gradle` | Yes | — |
| Ruby | `Gemfile` | — | — |
| PHP | `composer.json` | — | — |

"Yes" in the semantic column means symgraph has a tree-sitter grammar
and forge will extract functions, classes, imports, and env-var
reads for that language. Absent a route-extraction row, forge won't
detect HTTP endpoints in that language today — symbols and imports
still flow through correctly.

### Framework detection (from the dep set)

The `code` scanner infers a technology label by scanning the
dependency list against a curated priority table. See
[Scanners → code](scanners.md#code) for the full table. The
frameworks currently recognised:

- **Rust:** Axum, Actix-web, Rocket, Warp, Tonic (gRPC)
- **Python:** FastAPI, Django, Flask, Starlette
- **TypeScript / JavaScript:** Next.js, Nuxt, NestJS, Express,
  Fastify, Koa, Angular, React, Vue, Svelte
- **Go:** Gin, Echo, Fiber, gRPC
- **Java / Kotlin:** Spring Boot, Micronaut, Quarkus, Ktor
- **Ruby:** Rails, Sinatra

## Infrastructure and deployment

| Tool | Files | Scanner |
| --- | --- | --- |
| Docker | `Dockerfile`, `Dockerfile.*` | `docker` |
| Docker Compose | `docker-compose.yml` / `.yaml`, `compose.yml` / `.yaml` | `docker` |
| Kubernetes | `*.yaml` / `*.yml` with `kind:` + `apiVersion:` | `k8s` |
| AWS CloudFormation | `*.yaml` / `*.yml` / `*.json` with `AWSTemplateFormatVersion` or `AWS::` resources | `infra` |
| Terraform (AWS) | `*.tf` with `resource "aws_*"` blocks | `infra` |
| Terraform (GCP) | `*.tf` with `resource "google_*"` blocks | `infra` |
| OpenAPI | `*.yaml` / `*.yml` / `*.json` with `openapi:` or `swagger:` key | `infra` |

See [Scanners reference](scanners.md) for the exact Kubernetes kinds
and AWS/GCP resource types each scanner understands.

## CI/CD

| Tool | Files | Scanner |
| --- | --- | --- |
| GitHub Actions | `.github/workflows/*.yml` / `*.yaml` | `ci` |

Jobs become `stage` elements; `needs:` dependencies become stage
links; `environment: production` declarations drive the
[pipeline-env correlation pass](correlations.md).

## Version control

| Source | Scanner |
| --- | --- |
| Local git repository (branch refs, commit history) | `git` |
| `.github/CODEOWNERS`, `CODEOWNERS`, `docs/CODEOWNERS` | `git` |

The git scanner uses `gix` to read a local repository; it does not
hit the network.

## Editor integration (LSP)

`forge lsp` is a full LSP server built with `tower-lsp`. Copy-paste
configs for these editors are in
[`forge/EDITORS.md`](../../forge/EDITORS.md):

- Visual Studio Code
- Neovim
- Helix
- Emacs (`eglot` and `lsp-mode`)
- IntelliJ IDEA / RustRover (via the LSP4IJ plugin)
- Sublime Text (LSP package)
- Zed

## AI agents (MCP)

`forge mcp` is a Model Context Protocol server that exposes six tools
over stdio. Compatible with:

- Claude Code
- Cursor
- Windsurf
- Any other MCP-speaking client

Exposed tools: `forge_query`, `forge_render`, `forge_check`,
`forge_element_detail`, `forge_search`, `forge_validate`.

## Output formats

| Format | Command | Use case |
| --- | --- | --- |
| SVG | `forge build`, `forge generate` | Diagrams |
| HTML site | `forge generate` | Hosted docs |
| JSON | `forge export --format json` | Programmatic consumption |
| YAML | `forge export --format yaml` | Programmatic consumption |
| SARIF 2.1.0 | `forge check --format sarif` | GitHub Code Scanning |
| JSON (lint) | `forge check --format json` | Custom CI reporting |

## Import formats

| Format | Command | Notes |
| --- | --- | --- |
| PlantUML C4 | `forge import --source file.puml` | Structural C4 elements and relationships |
| Mermaid flowchart | `forge import --source file.mmd` | Nodes become containers, edges become relationships |

## Planned (not yet implemented)

These appear in [`DESIGN.md`](../../DESIGN.md) as future direction
but aren't in `main` today. Documenting unimplemented features is
how docs become lies, so they're listed here as aspirations, not
promises:

- **Force-directed layout** for large landscape / overview diagrams
- **Cross-compilation** and pre-built binaries for Linux / macOS /
  Windows
- **Tree-sitter-native route extraction** (upstream queries to
  symgraph, deleting the regex extractors)
- **More scanner languages**: Swift, R, Julia, Elixir, Haskell
- **More IaC tools**: Pulumi, CDK, Bicep, Helm charts
- **More CI systems**: GitLab CI, Jenkins, CircleCI, Buildkite, Drone
- **Event-stream schema discovery**: Protobuf, Avro, JSON Schema
- **APM / tracing integration**: auto-discover call graphs from
  Jaeger / Tempo / Datadog
- **Secret references** as first-class elements linked to consumers
  (currently only the env-var name is captured from
  `valueFrom: secretKeyRef`)

Track progress in the repo's open issues.

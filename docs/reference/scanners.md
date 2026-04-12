# Scanners

`forge analyze` runs a pipeline of seven scanners, each narrowly
focused on one class of files, followed by a cross-scanner
[correlate pass](correlations.md).

| # | Scanner | Source file | Default in `--scanners` |
| --- | --- | --- | --- |
| 1 | [`code`](#code) | `src/analyze/code.rs` | yes |
| 2 | [`semantic`](#semantic) | `src/analyze/semantic.rs` | yes |
| 3 | [`ci`](#ci) | `src/analyze/ci.rs` | yes |
| 4 | [`docker`](#docker) | `src/analyze/docker.rs` | yes |
| 5 | [`git`](#git) | `src/analyze/git.rs` | yes |
| 6 | [`k8s`](#k8s) | `src/analyze/k8s.rs` | yes |
| 7 | [`infra`](#infra) | `src/analyze/infra.rs` | yes |

Order is significant: `code` always runs before `semantic` so source
files get attributed to the right container; all scanners run before
`correlate` so it has a complete picture to work from.

Every element a scanner produces is tagged `inferred` plus a
scanner-specific tag (`inferred:code`, `inferred:semantic`, …). This
is what makes [merge mode](../user-guide/merge-mode.md) safe.

## `code` {#code}

**Purpose:** discover containers from package-manager manifests and
infer their technology from the declared dependency set.

**Reads:**

- `Cargo.toml` (Rust)
- `package.json` (npm/yarn/pnpm)
- `go.mod` (Go)
- `pyproject.toml`, `requirements.txt` (Python)
- `Gemfile` (Ruby)
- `composer.json` (PHP)
- `pom.xml`, `build.gradle`, `build.gradle.kts` (Java/Kotlin)
- `build.sbt` (Scala)

**Parsing:** delegated to `symgraph::extraction::manifest`, which uses
real `serde_json` / `toml` parsers (not string matching).

**Produces:** one `container` element per manifest. The id is the
slugified package name (or the containing directory if the manifest
doesn't declare one). Technology is inferred by scanning the dep set
against a curated framework priority list:

| Dep keyword | Technology label |
| --- | --- |
| `axum` | `Rust / Axum` |
| `actix-web` | `Rust / Actix` |
| `rocket` | `Rust / Rocket` |
| `warp` | `Rust / Warp` |
| `tonic` | `Rust / Tonic (gRPC)` |
| `fastapi` | `Python / FastAPI` |
| `django` | `Python / Django` |
| `flask` | `Python / Flask` |
| `starlette` | `Python / Starlette` |
| `next` | `{base} / Next.js` |
| `nuxt` | `{base} / Nuxt` |
| `@nestjs/core` | `{base} / NestJS` |
| `express` | `{base} / Express` |
| `fastify` | `{base} / Fastify` |
| `koa` | `{base} / Koa` |
| `@angular/core` | `{base} / Angular` |
| `react` | `{base} / React` |
| `vue` | `{base} / Vue` |
| `svelte` | `{base} / Svelte` |
| `github.com/gin-gonic/gin` | `Go / Gin` |
| `github.com/labstack/echo` | `Go / Echo` |
| `github.com/gofiber/fiber` | `Go / Fiber` |
| `google.golang.org/grpc` | `Go / gRPC` |
| `spring-boot-starter` | `Java / Spring Boot` |
| `org.springframework.boot` | `Java / Spring Boot` |
| `micronaut` | `Java / Micronaut` |
| `quarkus` | `Java / Quarkus` |
| `io.ktor` | `Kotlin / Ktor` |
| `rails` | `Ruby / Rails` |
| `sinatra` | `Ruby / Sinatra` |

The `{base}` for JS/TS is picked from the file type: `TypeScript` if a
`tsconfig.json` sits next to `package.json`, `Node.js` otherwise.

**Workspace handling:**

- Cargo `[workspace].members` (glob expansion: `crates/*`,
  `packages/**`)
- npm / pnpm / yarn `workspaces` array or `{ packages: [...] }`

Each member becomes its own container; the workspace root does **not**
become a container of its own.

## `semantic` {#semantic}

**Purpose:** parse source files with tree-sitter (via `symgraph`) and
extract: HTTP route endpoints, cross-container import relationships,
database/queue usage patterns, and env-var reads.

**Reads:** source files in the languages symgraph supports. See
[Languages and tools](languages-and-tools.md) for the full list.

**Container attribution:** each source file is matched to its owning
container via a path-prefix lookup against the `ContainerIndex` that
`code` populated. Deepest-matching directory wins.

**What it produces:**

1. **API endpoints** via framework-specific regex extractors for:
   - TS/JS: Express `.get('/path', …)`, NestJS `@Get('/path')`
   - Java/Kotlin: Spring `@GetMapping(…)`, JAX-RS `@Path(…)`
   - Go: `r.GET("/path", …)`, `http.HandleFunc(…)`
   - Rust: `#[get("/path")]`, Axum `.route("/path", get(handler))`
   - Python: Flask `@app.route(…)`, FastAPI `@app.get("/…")`, Django `path(…)`
2. **Cross-container imports.** Symgraph's `Import` nodes are
   resolved against known container ids; when an import path ends
   with a known container's name, forge emits an `imports`
   relationship.
3. **Inferred infra containers** from URL literals in source:
   `postgres://`, `mysql://`, `mongodb://`, `redis://` → a
   `_inferred_<kind>` container tagged `database`. `kafka`, `amqp://`,
   `sqs`, `sns` → an `_inferred_<kind>` container tagged `messaging`.
4. **Env var reads** stored on the owning container as
   `forge:env_reads`. Patterns recognised:
   - TS/JS: `process.env.FOO`, `process.env['FOO']`
   - Python: `os.getenv("FOO")`, `os.environ["FOO"]`
   - Rust: `std::env::var("FOO")`, `env::var("FOO")`
   - Go: `os.Getenv("FOO")`, `os.LookupEnv("FOO")`
   - Java/Kotlin: `System.getenv("FOO")`

   Only `SHOUT_CASE` identifiers are recorded — camelCase locals get
   filtered out.

## `ci` {#ci}

**Purpose:** parse GitHub Actions workflows into `pipeline` and
`stage` elements.

**Reads:** `.github/workflows/*.yml`, `.github/workflows/*.yaml`.

**Produces:**

- One `pipeline` element per workflow file; name pulled from the
  `name:` field, description from the `on:` triggers
- One `stage` element per job, parented to its pipeline
- `needs:` dependencies become `StageLink`s between stages
- Implicit sequential order when `needs:` is absent
- `environment: production` (scalar or `{ name: production, url: … }`)
  stored on the stage under the `environment` property — later picked
  up by the [correlate pass](correlations.md) to synthesise
  `Environment` elements
- A `gate "environment-protection"` element under any stage that uses
  the full `{ name:, url: }` form (GitHub's environment-protection
  syntax implies approval)

## `docker` {#docker}

**Purpose:** parse Dockerfiles and docker-compose files into
containers with runtime-context properties.

**Reads:**

- `Dockerfile`, `Dockerfile.*` anywhere in the tree
- `docker-compose.yml`, `docker-compose.yaml`, `compose.yml`,
  `compose.yaml` at the scan root

**From Dockerfiles:** one container per file, name derived from
directory or `Dockerfile.<variant>`, technology from the `FROM` image
(mapped through a known image → tech table), ports from `EXPOSE`.

**From docker-compose:** one container per service entry. Image is
mapped to a technology label (`postgres`, `mysql`, `redis`, `mongo`,
`elasticsearch`, `rabbitmq`, `kafka`, …) and the container gets a
`database` tag when the image matches a known data-store pattern. The
`environment:` block produces `forge:env_provides` (list form
`- FOO=bar` and map form `FOO: bar` both supported). `depends_on`
produces `depends on` relationships. Ports go into the `ports`
property.

If a container with the same slugified id already exists (because the
code scanner found it earlier), docker enriches the existing element
in place rather than creating a duplicate — image, ports, and
`env_provides` are merged.

## `git` {#git}

**Purpose:** infer branching strategy, contributor activity, and team
ownership from a local git repository.

**Reads:**

- Branch refs via `gix::Repository::references()`
- Recent commit history (up to 200 commits from HEAD) for contributor
  stats
- `.github/CODEOWNERS`, `CODEOWNERS`, `docs/CODEOWNERS` (first one
  that exists wins)

**Branching strategy inference:**

| Signals present | Strategy |
| --- | --- |
| `develop` + (`release/*` or `hotfix/*`) | git-flow |
| `main` + `feature/*`, no `develop` | trunk-based |
| `main` only | github-flow |
| Everything else | unknown |

Produces `branch` elements for the trunk and pattern branches, linked
with `branches from` and `merges into` relationships.

**CODEOWNERS:** rules are matched against each container's
`forge:source` property (set by `code`) using CODEOWNERS syntax —
leading `/` anchoring, trailing `/` directories, `crates/*`
single-segment wildcards, `src/**` recursive globs, `*.ext`
extension patterns. The **last** rule that matches a path wins, per
CODEOWNERS spec. Owners are normalised (`@org/team` → `team`,
`@alice` → `alice`) and grouped into `team` elements with an `owns`
list.

**Fallback:** when no CODEOWNERS file exists, the top 5 commit
authors are emitted as `team` elements with empty `owns`. This was
the original behaviour before CODEOWNERS support and is retained
as a heuristic for repos without ownership rules.

## `k8s` {#k8s}

**Purpose:** parse Kubernetes manifests into DeploymentNodes and
surface env-var context for correlation.

**Reads:** `*.yaml` / `*.yml` files anywhere in the tree that contain
`kind:` and either `apiVersion:` or `apps/v1`. Multi-document files
(`---` separators) are handled.

**Kinds recognised:**

| K8s `kind` | Produces |
| --- | --- |
| `Deployment`, `StatefulSet`, `DaemonSet` | `DeploymentNode` with id `k8s.<namespace>.<name>`, `technology` = `"<kind> (N replicas)"`, properties for `namespace`, `replicas`, `image` |
| `Service` | Enriches a matching DeploymentNode with `service_type` and `ports` |
| `Ingress` | Adds `ApiEndpoint`s with `host+path` to a matching DeploymentNode's API catalog |
| `ConfigMap` | Adds an `envConfig` entry with every data key |

**Env-var extraction.** For Deployment/StatefulSet/DaemonSet workloads,
every env var name from the first container's spec is collected and
stored in `forge:env_provides`. Sources:

- Direct: `env: - name: FOO value: ...`
- `valueFrom: configMapKeyRef:` and `valueFrom: secretKeyRef:` — only
  the `name:` is recorded
- `envFrom: configMapRef:` — expands to every key of the referenced
  ConfigMap if it was parsed earlier in the same scan

`env_provides` is written to **both** the DeploymentNode and any
existing Container whose slugified id matches the deployment's
metadata.name. This mirrors how `docker` enriches existing containers.

## `infra` {#infra}

**Purpose:** parse cloud infrastructure-as-code and API specs.

**Reads:**

- **CloudFormation:** `*.yaml` / `*.yml` / `*.json` files containing
  `AWSTemplateFormatVersion` or `AWS::`
- **Terraform:** `*.tf` files (any `resource "aws_*"` or
  `resource "google_*"` block)
- **OpenAPI:** `*.yaml` / `*.yml` / `*.json` files with `openapi:` or
  `swagger:` keys

**AWS resource mapping (CloudFormation and Terraform both supported):**

| Resource | Element kind | Technology |
| --- | --- | --- |
| `AWS::Lambda::Function` / `aws_lambda_function` | Container | AWS Lambda |
| `AWS::ECS::Service` / `AWS::ECS::TaskDefinition` / `aws_ecs_*` | Container | AWS ECS |
| `AWS::EC2::Instance` / `aws_instance` | DeploymentNode | AWS EC2 |
| `AWS::RDS::DBInstance` / `AWS::RDS::DBCluster` / `aws_db_instance` / `aws_rds_cluster` | Container (tagged database) | AWS RDS |
| `AWS::DynamoDB::Table` / `aws_dynamodb_table` | Container (tagged database) | AWS DynamoDB |
| `AWS::ElastiCache::*` / `aws_elasticache_*` | Container (tagged database) | AWS ElastiCache |
| `AWS::S3::Bucket` / `aws_s3_bucket` | Container | AWS S3 |
| `AWS::SQS::Queue` / `aws_sqs_queue` | Container (tagged messaging) | AWS SQS |
| `AWS::SNS::Topic` / `aws_sns_topic` | Container (tagged messaging) | AWS SNS |
| `AWS::ApiGateway::RestApi` / `AWS::ApiGatewayV2::Api` / `aws_api_gateway_*` | Container | AWS API Gateway |
| `AWS::EKS::Cluster` / `aws_eks_cluster` | DeploymentNode | AWS EKS |
| `AWS::CloudFront::Distribution` / `aws_cloudfront_distribution` | Container | AWS CloudFront |
| `AWS::Kinesis::Stream` | Container (tagged messaging) | AWS Kinesis |

**GCP resource mapping (Terraform only):**

| Resource | Element kind | Technology |
| --- | --- | --- |
| `google_cloud_run_service` | Container | Google Cloud Run |
| `google_sql_database_instance` | Container (tagged database) | Google Cloud SQL |
| `google_container_cluster` | DeploymentNode | Google GKE |
| `google_pubsub_topic` | Container (tagged messaging) | Google Pub/Sub |
| `google_storage_bucket` | Container | Google Cloud Storage |

**OpenAPI specs** produce `ApiEndpoint` entries in the container's
catalog, keyed by method + path. When the filename suggests an owning
service (e.g. `payments-openapi.yaml`), endpoints attach to that
container; otherwise they land on a synthetic `_inferred_api` element.

## `correlate`

Not a scanner per se — runs after all scanners have populated the
model. Joins facts from multiple sources into concrete relationships.
See [Correlations](correlations.md) for the three current passes.

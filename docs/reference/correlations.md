# Correlations

After every scanner runs, `forge analyze` executes a final pass that
joins scanner-local facts into concrete relationships. This is where
individual narrow heuristics combine into architecturally meaningful
edges.

Three passes fire today, in order. Each is additive: later passes
don't undo earlier ones, but they do skip cases an earlier pass
already handled.

## 1. Exact env var match

**Source:** `src/analyze/correlate.rs::correlate_env_vars`

**Input:** every element with a `forge:env_reads` property (set by
`semantic`) and every element with a `forge:env_provides` property
(set by `docker` or `k8s`).

**Rule:** for every (reader, provider) pair where reader ≠ provider
and their variable sets share at least one name, emit a
`uses (VAR1, VAR2, …)` relationship from reader to provider.

**Dedup:** coexists with other edges between the same pair (e.g.
docker-compose `depends_on` already emits a `depends on` edge). Only
correlation edges (label prefix `uses (`) are deduped against each
other.

**Side effect:** if the provider is tagged `database`, every
relationship from the same reader to an `_inferred_*` element is
dropped. This collapses the "semantic saw `postgres://` literal →
created _inferred_postgresql" ghost container into the real concrete
edge.

**Example.** A service reading `DATABASE_URL` in source, and a
postgres compose service declaring `DATABASE_URL=postgres://db/app`
in its environment block:

```
api → db : uses (DATABASE_URL)
```

## 2. Connection-string fallback

**Source:** `src/analyze/correlate.rs::correlate_connection_strings`

**Input:** every element with `forge:env_reads`.

**Rule:** when a reader has an env var that looks like a well-known
data-store URL, but no exact-match correlation fired for it, link
the reader to the first (lexicographically smallest id) existing
Container tagged `database` or `messaging` whose technology / name /
id contains the expected kind substring.

**Recognised hints:**

| Kind | Env var names |
| --- | --- |
| `postgres` | `DATABASE_URL`, `DB_URL`, `POSTGRES_URL`, `POSTGRESQL_URL`, `PG_URL`, `PGHOST`, `PGDATABASE` |
| `mysql` | `MYSQL_URL`, `MYSQL_HOST`, `MYSQL_DATABASE`, `MARIADB_URL` |
| `redis` | `REDIS_URL`, `REDIS_HOST`, `CACHE_URL`, `CACHE_REDIS_URL` |
| `mongo` | `MONGO_URL`, `MONGODB_URI`, `MONGO_URI`, `MONGODB_URL` |
| `elasticsearch` | `ELASTICSEARCH_URL`, `ELASTIC_URL`, `ES_URL` |
| `kafka` | `KAFKA_BROKERS`, `KAFKA_URL`, `KAFKA_BOOTSTRAP` |
| `rabbitmq` | `AMQP_URL`, `RABBITMQ_URL`, `RABBITMQ_HOST` |

**Matching:** a container is a candidate for `kind` if any of its
`technology`, `name`, or `id` (case-insensitive) contains the kind
string. Example: a container with `technology "PostgreSQL 16"` is a
valid postgres target; a container with `name "Cache"` and
`technology "Redis"` is a valid redis target.

**Skipped when already satisfied:** if a correlation edge already
exists from the reader to some container of the same kind (from
pass 1), the fallback is skipped for that variable. No duplicates.

**Side effect:** same as pass 1 — stale `_inferred_*` edges from the
same reader are dropped once a concrete target exists.

**Why this matters.** Real postgres services only declare
`POSTGRES_PASSWORD`, not `DATABASE_URL`. Pass 1 alone can't link a
reader of `DATABASE_URL` to that service. This pass closes the gap
by heuristic: if you read a well-known URL env var, the nearest
database-tagged service is almost certainly what you're talking to.

**Example.**

- `billing` reads `DATABASE_URL` in source (`std::env::var("DATABASE_URL")`)
- `db` is a docker-compose postgres service declaring only
  `POSTGRES_PASSWORD` and tagged `database` with technology `PostgreSQL 16`
- Pass 1 finds no exact match
- Pass 2 sees `DATABASE_URL` in the reads set, looks up kind
  `postgres`, finds `db` (technology contains `postgres`), emits
  `billing → db : uses (DATABASE_URL)`.

## 3. Pipeline stages → environments

**Source:** `src/analyze/correlate.rs::correlate_pipeline_environments`

**Input:** every `Stage` element with an `environment` property (set
by `ci` from GitHub Actions `environment: prod` declarations), plus
every `DeploymentNode` with a `namespace` property (set by `k8s`).

**Rule:**

1. For each unique environment name mentioned by any stage, create
   (or find) an `Environment` element with id `env.<slug(name)>`.
2. Link each stage to its environment with a `deploys to`
   relationship.
3. For each environment whose name matches a k8s namespace, link the
   environment to every DeploymentNode in that namespace with a
   `hosts` relationship.

**Result:** a query-able chain `stage → env → deployment`. The
pipeline view can then render the full delivery path, from CI job to
running pod.

**Example.**

- `.github/workflows/deploy.yml` has a `deploy-prod` job with
  `environment: { name: prod, url: … }`
- `deploy/billing.yaml` has a `kind: Deployment` with
  `metadata.namespace: prod`
- Pass 3 creates `env.prod`, emits:
  - `deploy.deploy-prod → env.prod : deploys to`
  - `env.prod → k8s.prod.billing : hosts`

The `Environment` element is synthetic (created by correlate, tagged
`inferred:correlate`) and survives [merge mode](../user-guide/merge-mode.md)
the same way any other inferred element does.

## Provenance {#provenance}

Every element produced by a scanner or by correlate carries at least
two tags:

- `inferred` — generic marker distinguishing scanner output from
  hand-authored content
- `inferred:<source>` — scanner name or `correlate` for the cross-
  scanner pass

Some elements also get a `forge:source` property pointing at the
originating file. This is what the [CODEOWNERS matcher in `git`](scanners.md#git)
uses to attribute containers to teams.

Merge mode uses the `inferred` tag to decide whether an element is
analyzer-owned (safe to refresh) or user-owned (must be preserved).
See [Merge mode](../user-guide/merge-mode.md) for the full semantics.

## Extending the correlate pass

Adding a new cross-scanner pass is a small, contained change:

1. Write a new function in `src/analyze/correlate.rs` that reads
   from the model and appends to `model.relationships`.
2. Add a call to it from the top-level `run(model)` function.
3. Add unit tests in the same file using tiny hand-built `Model`
   fixtures.
4. Add an end-to-end fixture under
   `forge/tests/fixtures/analyze/<your-fixture>/` and a matching
   test in `fixture_tests.rs` that exercises the full `analyze()`
   pipeline.

Future passes that would make sense:

- **Secret references.** `k8s` captures `valueFrom: secretKeyRef`
  env names; a pass that creates `Secret` pseudo-elements and links
  consumers to them would surface "this service depends on this
  secret" in deployment views.
- **Ingress host → API catalog URLs.** `k8s` already extracts
  ingress rules; propagating the `host:` into `ApiEndpoint.description`
  would give generated docs proper base URLs.
- **APM trace-derived call graphs.** Import Jaeger / Tempo / Datadog
  service maps and correlate against the discovered containers,
  upgrading guessed `imports` edges into observed `calls` edges.

See [`forge/src/analyze/correlate.rs`](../../forge/src/analyze/correlate.rs)
for the current implementation.

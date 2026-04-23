# `forge analyze` — scanner reference

The analyzer is a pipeline of single-responsibility scanners that each contribute elements, relationships, and provenance tags to a shared `Model`. A post-pass correlates cross-scanner facts (e.g. env-var providers to consumers) and synthesises missing structure (System wrappers, default Views).

## Scanners

| Name | Detects | Typical inputs |
|---|---|---|
| `code` | Containers from build manifests; languages. | `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `pom.xml`, `Gemfile`, `composer.json` |
| `semantic` | Language-specific components (imports, routes) and cross-container refs. Runs after `code`. | Source files under each detected container. |
| `ci` | Pipelines, stages, gates. | `.github/workflows/*.yml`, `.gitlab-ci.yml`, `Jenkinsfile`, CircleCI, Azure DevOps. |
| `docker` | Containers via Dockerfile; technology via base image; dependencies via `depends_on`; env-var provides. | `Dockerfile`, `Dockerfile.*`, `docker-compose.yml`. |
| `git` | Repository elements, branch strategies. | `.git/` metadata. |
| `k8s` | DeploymentNodes, container instances. | Kubernetes manifests, Helm charts under `charts/`. |
| `infra` | Deployment topology. | Terraform (`*.tf`), CloudFormation (`*.yml` with `Resources:`). |

**Order matters.** `code` must run before `semantic` — the former creates the container elements that `semantic` attributes components to. `forge analyze` enforces this even if the caller passes `--scanners semantic,code`.

## Useful CLI combos

```bash
# Smoke test a single scanner in isolation.
forge analyze . --scanners code --dry-run

# Write an authored-friendly file, then edit and re-merge.
forge analyze . --out architecture.forge
# ...edits...
forge analyze . --merge architecture.forge

# Quiet the obvious vendored-code noise.
forge analyze . --exclude third_party --exclude generated
```

## Provenance

Every inferred element carries `inferred:<scanner>` in `tags`. Hand-authored content has no such tag. The `--merge` flow refreshes *only* inferred elements; authored prose and custom views survive. When debugging "the analyzer keeps overwriting my edits", first check that the element you're editing has no `inferred:*` tag — if it does, remove the tag to pin it.

## Gotchas

- **Monorepos with shared `package.json`**: the code scanner treats a top-level `package.json` as a single container. Split manually or point `forge analyze` at each workspace root.
- **Polyglot containers** (e.g. a Dockerfile that builds Go and ships static JS): the `docker` scanner picks the last language it sees in `FROM` lines. Add `technology "Go + JS"` manually in the `.forge` file.
- **Private registries**: `image: ghcr.io/org/custom:latest` returns `None` from `image_to_technology` — the container is created but has no technology tag. Fine to fix post-hoc.
- **No tests in corpus**: before trusting a run, glance at the `inferred:semantic` count. If it's `0`, you got a topology but no components — usually worth re-running with broader scanners or investigating the source layout.

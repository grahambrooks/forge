---
name: model-repository
description: Produce a Forge architecture model (C4 + process + deployment) from a repository. Use this when the user asks to "model this repo", "diagram the architecture", "generate a C4 model", "create a forge model", "analyze this codebase architecturally", "produce an architecture doc site", or any variant of inferring architecture from source. Invokes `forge analyze` and the Forge MCP server.
---

# Model a Repository with Forge

This skill turns a codebase on disk into a Forge model — a single `.forge` DSL file describing containers, components, relationships, pipelines, deployments, and tech stack — and then loads it into the Forge MCP server so you can query and render it interactively.

**The model is the artefact.** Everything else (diagrams, lint, docs site) is regenerated from it. Never hand-edit generated diagrams; edit the `.forge` file and rebuild.

## Prerequisites

- The `forge` MCP server (`mcp__forge__*` tools) should be configured in the user's Claude Code session. The quickest way to confirm it's available is to attempt `forge_analyze` directly — a missing server will surface as a tool-call error, which is a more reliable signal than `forge --version`.
- If the MCP tools aren't registered, the `forge` CLI is the fallback: every MCP tool has a `forge <subcommand>` equivalent. Use `Bash` to run it, but prefer the MCP when both are available.
- Only check `forge --version` manually when the MCP is demonstrably missing and you're trying to diagnose why.

If the user's repo already has a `.forge` file, skip the analyze step and jump to "Load the model" — the existing file is the source of truth.

## Workflow

### 1. Analyze

Run the analyzer. Prefer the MCP tool `forge_analyze` when available — it loads the result straight into the server, so subsequent queries have no disk round-trip.

```
forge_analyze {
  "path": "<repo root>",
  "out":  "architecture.forge"    // omit to keep the model in-memory only
}
```

CLI fallback:

```bash
forge analyze <repo> --out architecture.forge
```

**Scanner hints** — only narrow `scanners` if the full list is clearly wasteful (e.g. `scanners: "code,docker"` for a tiny library with no CI). The default set runs fast and the extra signal is usually worth it.

### 2. Orient yourself in the model

Before drilling in, always call `forge_overview` first. It returns:

- name, source, totals
- counts by element kind (Container, Component, Pipeline, …)
- top-level systems and containers
- list of views

A model with **zero containers** almost always means the code scanner got confused by the project layout — tell the user, suggest they pass `--exclude` for vendored dirs, and stop. Don't invent elements.

### 3. Validate quality

Run `forge_check` at `severity: "info"`. Common early-life issues:

- `orphan-container` — a container with no inbound or outbound edges; usually a scanner mistake or a genuinely unreachable service.
- `missing-technology` — container has no `technology` field. If you can infer it (Dockerfile base image, `package.json`, `Cargo.toml`), edit the `.forge` file and rerun `forge_reload`.
- `unresolved-reference` — a relationship targets an id that doesn't exist. Usually a correlate-pass bug; report it rather than patching.

### 4. Present findings

Give the user a **summary they can act on**, not a data dump:

- One sentence on what was detected (e.g. "3 services written in Go and Python, deployed to Kubernetes, built by GitHub Actions").
- 3–5 bullets of interesting facts from the overview (top containers, tech mix, deployment target).
- Any check violations worth fixing.
- A prompt to next step: render a view, generate a site, or edit the `.forge`.

### 5. Render and/or generate

- `forge_list_views` → `forge_render { view_key: "Containers" }` to show any single view as SVG.
- `forge generate --source architecture.forge --out _site` for a navigable static site (use the Bash tool; there is no MCP equivalent).
- `forge serve --source architecture.forge` for live-reload during editing.

## When analyze gets it wrong

The analyzer is conservative but not omniscient. If the user says something like "you missed the billing service" or "that isn't a database":

1. Confirm by reading the actual file the analyzer tagged (provenance is in `tags` — `inferred:code`, `inferred:docker`, etc., and `properties.dockerfile` / `properties.source` hold the path).
2. If the scanner could reasonably have caught it, this is a bug worth noting — mention it plainly.
3. Otherwise, edit the `.forge` file directly: add the missing container, remove the misclassification, or rewrite a relationship. Call `forge_reload` afterwards.

Hand-authored edits are **preserved on re-analyze** if you use `forge analyze --merge architecture.forge` — only elements tagged `inferred` are refreshed.

## Tool quick reference

| You want to… | Use |
|---|---|
| Infer a model from a repo | `forge_analyze` |
| Reload after external edits | `forge_reload` |
| Know what's in the model | `forge_overview` |
| Enumerate views | `forge_list_views` |
| Find an element | `forge_search` / `forge_query` |
| Drill into one element | `forge_element_detail` |
| Render a view as SVG | `forge_render` |
| Lint the architecture | `forge_check` |
| Sanity-check a DSL snippet | `forge_validate` |

See `references/analyze-options.md` for the full scanner list and tuning notes.

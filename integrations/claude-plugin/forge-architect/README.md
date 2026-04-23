# forge-architect

A Claude Code plugin that gives Claude the ability to **model, review, and author software architectures** using [Forge](https://github.com/grahambrooks/forge) — a unified DSL for C4 structure, CI/CD process, deployment, data, and trust-boundaries.

## What it provides

- **MCP server** — `forge mcp` registered as the `forge` server. Exposes ten tools:
  `forge_analyze`, `forge_reload`, `forge_overview`, `forge_list_views`,
  `forge_query`, `forge_render`, `forge_check`, `forge_element_detail`,
  `forge_search`, `forge_validate`.
- **Three skills**:
  - `model-repository` — analyze a codebase into a `.forge` model.
  - `architecture-review` — interrogate an existing model.
  - `forge-dsl` — author or edit `.forge` files.

## Prerequisites

- `forge` binary on `PATH`. Build from source:
  ```bash
  cargo install --path forge --locked
  ```
  or use a release build (`forge/target/release/forge`) — symlink into `~/.local/bin`.

## Installation (local dev)

From this repo:

```bash
# Symlink into your user plugin dir so Claude Code picks it up.
ln -s "$PWD/integrations/claude-plugin/forge-architect" \
      "$HOME/.claude/plugins/data/forge-architect-local"
```

Then reload Claude Code. The three skills will appear in `/skills` and the `forge` MCP server will be available.

Alternatively, publish via a marketplace entry pointing at this subdirectory.

## Usage patterns

| User asks… | Skill that fires | Primary tool |
|---|---|---|
| "Model this repo as C4." | `model-repository` | `forge_analyze`, `forge_overview` |
| "Audit our architecture." | `architecture-review` | `forge_check`, `forge_query` |
| "Explain how X talks to Y." | `architecture-review` | `forge_element_detail` |
| "Add a Redis cache to the model." | `forge-dsl` | `Edit` + `forge_validate` + `forge_reload` |

## Design notes

- **One-shot model loads**: the MCP holds a single `Model` in memory. `forge_analyze` replaces it (or merges with `merge: true`); `forge_reload` rereads the `.forge` source.
- **Nothing is silently destructive**: authored content tagged anything other than `inferred:*` survives `--merge`.
- **Output is SVG-first**: `forge_render` returns SVG strings; write them to a file via `Write` rather than pasting them into chat.
- **CLI parity**: every MCP tool has a `forge <subcommand>` CLI equivalent. Skills prefer MCP but fall back to `Bash` when a tool isn't available.

## Not yet covered

- Custom `.forge-rules` aren't loaded by the MCP; use `forge check --rules` via the CLI.
- PNG/PDF export — tracked in the main repo backlog.
- Animated views render via SVG CSS keyframes; interactive presentation mode needs the `forge serve --present` CLI.

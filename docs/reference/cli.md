# CLI reference

Every `forge` subcommand, every flag, and what each one does. Flags are
given with both short (`-s`) and long (`--source`) forms when available.

Global help: `forge --help`. Per-command help: `forge <command> --help`.

## `forge build`

Parse a `.forge` file and render SVG diagrams for one or more views.

```
forge build [--source FILE] [--view KEY] [--out DIR] [--style filled|outline]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `--view` | _all views_ | Render only the view with this key; omit to render every view in the model |
| `-o`, `--out` | `_site/diagrams` | Output directory for SVGs |
| `--style` | `filled` | `filled` for shaded colourful diagrams, `outline` for mono line-art |

Example — render a single view in outline style:

```bash
forge build --source architecture.forge --view Containers --style outline --out out/
```

## `forge check`

Lint a model against the built-in architectural rules plus any custom
`.forge-rules`. See [Linter rules](linter-rules.md) for the full rule
catalog.

```
forge check [--source FILE] [--severity LEVEL] [--format FMT] [--rules FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `--severity` | `warning` | Minimum severity to report: `info`, `warning`, or `error` |
| `-f`, `--format` | `text` | Output format: `text`, `json`, or `sarif` (SARIF 2.1.0 for GitHub Code Scanning) |
| `--rules` | _none_ | Path to a `.forge-rules` file with custom rules |

Exit code is non-zero if any violation at or above the severity
threshold fires. See [Linting](../user-guide/linting.md) for the
tutorial version.

## `forge analyze`

Scan a codebase and write a `.forge` file. Seven scanners run in order
(`code`, `semantic`, `ci`, `docker`, `git`, `k8s`, `infra`), followed by
a correlate post-pass. Use `--merge` when re-running against a
hand-edited file.

```
forge analyze [PATHS...] [--out FILE] [--scanners LIST] [--exclude DIR]...
              [--dry-run] [--merge FILE]
```

| Flag / positional | Default | Description |
| --- | --- | --- |
| `PATHS...` | `.` | Directories to scan. Multiple roots can be passed |
| `-o`, `--out` | `forge.forge` | Output `.forge` file |
| `--scanners` | `code,semantic,ci,docker,git,k8s,infra` | Comma-separated scanner allow-list. See [Scanners](scanners.md) |
| `--exclude` | _none (in addition to defaults)_ | Directory names to skip. Repeatable. Always skipped: `node_modules`, `target`, `.git`, `vendor`, `dist`, `__pycache__` |
| `--dry-run` | off | Print the output to stdout instead of writing a file |
| `--merge` | off | Merge fresh analysis into an existing `.forge`. See [Merge mode](../user-guide/merge-mode.md) |

Examples:

```bash
# First-run bootstrap
forge analyze --out architecture.forge

# Narrow to manifest discovery only
forge analyze --scanners code --out architecture.forge

# Safe re-run that preserves hand-edits
forge analyze --merge architecture.forge --out architecture.forge

# Peek at what the scanners would produce
forge analyze --dry-run .
```

## `forge generate`

Turn a `.forge` file into a self-contained static HTML site. See
[Generating docs](../user-guide/generating-docs.md) for the walkthrough.

```
forge generate [--source FILE] [--out DIR] [--title STR] [--base-url PATH]
               [--style filled|outline] [--baseline FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `-o`, `--out` | `_site` | Output directory |
| `--title` | _model name_ | Site title; defaults to the top-level `forge "Name"` |
| `--base-url` | `/` | URL prefix for deploys under a subpath (e.g. `/my-repo/` for GitHub Pages) |
| `--style` | `filled` | Diagram style for rendered SVGs |
| `--baseline` | _none_ | Baseline `.forge` file to diff against; changes are highlighted in green (added) and amber (modified) |

## `forge export`

Export the parsed model as structured data. Useful for feeding the
model into external tools (dashboards, tests, ad-hoc queries).

```
forge export [--source FILE] [--format json|yaml] [--out FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `-f`, `--format` | `json` | Output format: `json` or `yaml` |
| `-o`, `--out` | _stdout_ | Output file; omit to print to stdout |

## `forge import`

Import a model from PlantUML C4 or Mermaid flowchart notation. Output is
a `.forge` file ready to refine by hand.

```
forge import --source FILE [--out FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | _required_ | Input file (`.puml`, `.mmd`, or any text the importer recognises) |
| `-o`, `--out` | _stdout_ | Output `.forge` file; omit to print |

## `forge watch`

Watch a `.forge` file and rebuild SVGs on every save. See
[Live preview](../user-guide/live-preview.md).

```
forge watch [--source FILE] [--out DIR] [--style filled|outline] [--baseline FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `-o`, `--out` | `_site` | Output directory |
| `--style` | `filled` | Diagram rendering style |
| `--baseline` | _none_ | Baseline `.forge` for diff highlighting |

## `forge serve`

Start a local HTTP preview server with live reload over Server-Sent
Events. See [Live preview](../user-guide/live-preview.md).

```
forge serve [--source FILE] [--out DIR] [--style filled|outline]
            [--port PORT] [--baseline FILE] [--present]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Input `.forge` file |
| `-o`, `--out` | `_site` | Output directory for rendered assets |
| `--style` | `filled` | Diagram rendering style |
| `-p`, `--port` | `4000` | HTTP port |
| `--baseline` | _none_ | Baseline `.forge` for diff highlighting |
| `--present` | off | Presentation mode for animated views (keyboard-driven frame stepping) |

## `forge mcp`

Start the Model Context Protocol server over stdio, exposing six tools
that let AI agents (Claude Code, Cursor, Windsurf, others) query the
model.

```
forge mcp [--source FILE]
```

| Flag | Default | Description |
| --- | --- | --- |
| `-s`, `--source` | `forge.forge` | Model to serve |

Tool names exposed by the MCP server: `forge_query`, `forge_render`,
`forge_check`, `forge_element_detail`, `forge_search`,
`forge_validate`.

## `forge lsp`

Start the Language Server Protocol server over stdio. Provides
diagnostics, hover, completion, and go-to-definition to any LSP-aware
editor.

```
forge lsp
```

No flags. For editor configurations, see
[`forge/EDITORS.md`](../../forge/EDITORS.md).

## Exit codes

All commands use standard Unix exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Success, or (for `check`) no violations at or above threshold |
| `1` | Runtime error (parse failure, file not found, I/O error) |
| `2` | Linter violations at or above the configured severity |

# Editor integration (LSP)

Forge ships a full Language Server Protocol implementation. Any editor
that speaks LSP can give you live diagnostics, hover popups, symbol
completion, document outlines, and go-to-definition while you edit
`.forge` files — no plugin required beyond the generic LSP client
your editor already has.

This page describes what the server actually does. For copy-paste
editor configurations (VS Code, Neovim, Helix, Emacs, IntelliJ,
Sublime Text, Zed), see [`forge/EDITORS.md`](../../forge/EDITORS.md).

## Starting the server

```bash
forge lsp
```

That's it — no flags, no ports. The server reads LSP messages on
stdin and writes responses on stdout. Your editor launches it as a
subprocess on demand; you don't run it yourself.

Under the hood: `tower-lsp` on top of `tokio`. The server is
stateless between editor sessions; it re-parses each document on
every change and rebuilds the in-memory index.

## Feature summary

| Feature | LSP method | What it does |
| --- | --- | --- |
| Diagnostics | `textDocument/publishDiagnostics` | Parse errors and lint findings surfaced as you type |
| Hover | `textDocument/hover` | Tooltip with element details or keyword docs |
| Completion | `textDocument/completion` | DSL keywords + element ids in scope |
| Go to definition | `textDocument/definition` | Jump from a reference to the element declaration |
| Document symbols | `textDocument/documentSymbol` | Outline view of systems, containers, views |

The server advertises these capabilities at `initialize` time. If
your editor doesn't call one, the server does nothing unexpected —
unsupported features are silently absent.

## Diagnostics

Every time the document changes, the server re-parses it and runs
every built-in [linter rule](../reference/linter-rules.md) at the
`info` severity level. Findings are published as diagnostics with:

- **Source:** `forge`
- **Code:** the rule name (e.g. `missing-descriptions`,
  `dependency-cycles`)
- **Severity:** `error`, `warning`, or `information` mapping the
  linter's severity levels directly
- **Range:** pinned to the offending element's declaration line
  when forge can find it; line 0 column 0 otherwise

### Parse errors

A broken file produces exactly one diagnostic at the first parse
failure:

```
[error] expected identifier at line 12 col 34
```

While the file is broken, the model is `None`, so hover, completion
of element ids, and go-to-definition return nothing. Keyword
completion and keyword hover continue to work because they don't
depend on a parsed model.

### Architectural lint diagnostics

Once the file parses, the server runs the full check pass and
surfaces every violation as a diagnostic. A freshly hand-authored
model usually has a handful of `missing-descriptions` and
`missing-technology` warnings; you'll watch them disappear as you
fill in fields.

The LSP always runs checks at `info` severity (the most permissive
setting), so you'll see `chatty-coupling` and `empty-views`
findings that a CI `forge check` with the default `warning`
threshold would suppress. This is intentional — the editor is the
place to catch soft issues early.

See [Linter rules reference](../reference/linter-rules.md) for the
eight built-in rules and how to fix each.

## Hover

Hover over an identifier to get contextual information.

### Hovering on an element id

Returns a Markdown panel with the element's kind, display name,
description, technology, and tags. Example:

```markdown
**Payment API** (Container)

REST + gRPC gateway for the payments platform

*Technology:* Rust / Actix
*Tags:* core, pci
```

Works on both fully-qualified ids (`payments.api`) and local ids
(`api` when you're inside the `payments` system block). The hover
handler walks the model's element map to find a match.

### Hovering on a DSL keyword

Returns a one-line description of the keyword. Example — hover
over `container`:

```markdown
**container** — A deployable unit within a system (service, database, app).
```

The server recognises 24 keywords: the top-level blocks (`model`,
`process`, `deployment`, `views`, `techStack`, `dataModel`,
`trustBoundaries`, `teams`, `docs`), element kinds (`person`,
`system`, `container`, `component`, `pipeline`, `stage`, `gate`,
`node`, `instance`, `entity`, `boundary`, `team`, `branch`), and a
handful of binding keywords.

## Completion

Triggered automatically on `"`, ` `, or `{` (configurable per
editor; your editor may also trigger it on every keystroke).
Produces two kinds of items:

### Keyword completions

Every known DSL keyword is offered as a completion item with kind
`Keyword` and the detail `"Forge DSL keyword"`. The full list is
currently 55 keywords including block names, element kinds, view
kinds (`systemContext`, `container` as a view, `pipelineView`,
`deploymentView`, `techStackView`, `branchingView`, `dataModelView`,
`trustBoundaryView`, `teamView`), and structural keywords
(`description`, `technology`, `tags`, `include`, `autoLayout`,
`title`, etc.).

### Element-id completions

Every element in the currently-parsed model is offered as a
`Reference` completion, with:

- **label:** the local id (`api`, not `payments.api`)
- **detail:** the element kind and display name (`Container —
  Payment API`)
- **documentation:** the element's description as plain-text
  documentation

This is what makes writing relationships bearable: type `api ->` and
autocomplete suggests every container in scope.

### Completion limitations

- No context-awareness. The server offers every known element id
  everywhere, not just the ones valid at the current cursor
  position. Editors with fuzzy filtering handle this well in
  practice.
- No snippet insertion. Completions are plain identifiers. Snippets
  for common blocks (`container "..."{ ... }`) would be a nice
  addition — see the [roadmap](../roadmap.md) for future work.
- Unknown when the file is broken. If parsing fails, element-id
  completions are empty until you fix the syntax error.

## Go to definition

Right-click → "Go to Definition" on an element reference jumps
back to the line that declared it.

How it works: after each parse, the server walks the source text
for every element's *display name* and records the first byte
position where the name appears. Go-to-def reads that position
directly.

### Limitations

- **Display-name-based lookup.** If two elements share the same
  display name, go-to-def picks the first one. In practice this
  isn't a problem because forge ids are usually unique, but it's
  worth knowing.
- **Same-file only.** The server tracks one file at a time. If a
  reference resolves to an element imported via `!include`, go-to
  -def won't follow it across files. Multi-file support is
  [roadmap work](../roadmap.md).
- **Position is start-of-name.** The LSP reports a zero-width
  range at the name's start rather than a full range. Editors
  handle this fine but some show a point cursor rather than a
  selection.

## Document symbols

The server exposes an outline view with two kinds of entries:

1. **Top-level elements** — every element without a parent. Systems,
   persons, top-level containers, pipelines, repositories, and
   deployment nodes. Each entry has a kind-specific LSP
   `SymbolKind`:

   | Forge kind | LSP SymbolKind |
   | --- | --- |
   | Person | `Interface` |
   | System | `Module` |
   | Container | `Class` |
   | Component | `Method` |
   | Pipeline | `Function` |
   | Stage | `Event` |
   | Repository | `File` |
   | Branch | `Variable` |
   | DeploymentNode | `Package` |
   | Other | `Object` |

2. **Views** — every entry in the model's `views { … }` block,
   reported as `SymbolKind::Namespace`.

This powers the "go to symbol" / "outline" panel in every major
editor. VS Code shows the list in its breadcrumb bar; Neovim's
`vim.lsp.buf.document_symbol()` uses it; Zed's command palette
search (`@symbol`) queries it.

## What the server doesn't do (yet)

- **Code actions / quick fixes.** LSP supports `textDocument/codeAction`
  for things like "add a description to this container." Forge
  doesn't implement it yet. When it does, it'll probably be driven
  by the same template system proposed for the MCP
  `forge_suggest_fix` tool — see [roadmap item 6](../roadmap.md#6-write-capable-mcp-tools).
- **Rename.** Renaming an id across every reference. Useful but
  not implemented.
- **References.** "Find all usages of this element." Not implemented.
- **Formatting.** `textDocument/formatting` for auto-indentation
  and canonical spacing. Not implemented — the DSL is simple enough
  that most editors handle formatting with their default
  whitespace rules.
- **Semantic highlighting.** Better token colouring than a regex-
  based TextMate grammar can provide. Not implemented; your editor
  probably has a grammar-based forge highlighter already, or needs
  one.
- **Workspace-level features.** Cross-file go-to-definition,
  workspace-wide rename, multi-file diagnostics. Not implemented;
  the server is strictly single-file.

## Troubleshooting

### "My editor doesn't show any forge features"

Check that:

1. `forge lsp` works on the command line. It should start silently
   and wait for input — press Ctrl-C to exit.
2. Your editor's generic LSP client is configured to start `forge
   lsp` on `.forge` files. See [`forge/EDITORS.md`](../../forge/EDITORS.md)
   for per-editor setup.
3. The editor's LSP log (usually under a "LSP: show log" or
   "Language server output" command) shows an `initialize` request
   and a capabilities response.

### "Diagnostics appear but element hover is empty"

Probably a parse error upstream. The server falls back to keyword
hover when the model is `None`, but element hover depends on a
parsed model. Fix the parse error shown by the diagnostics and
hover will start working again.

### "Go-to-definition jumps to the wrong line"

The location lookup is display-name-based. If you renamed an
element but left references pointing at the old name, the server
can't resolve them. Save the file to trigger a re-parse; if the
issue persists, check that the element's name is unique in the
file.

### "Completion suggests ids I don't want"

All known element ids are offered unconditionally. Rely on your
editor's fuzzy filter, or use more specific prefixes in your ids
so the filter narrows quickly.

## See also

- [`forge/EDITORS.md`](../../forge/EDITORS.md) — per-editor setup
  for VS Code, Neovim, Helix, Emacs, IntelliJ, Sublime Text, Zed
- [Live preview](live-preview.md) — pair the LSP with
  `forge serve` for a browser-rendered live diagram alongside your
  editor
- [Linter rules](../reference/linter-rules.md) — the diagnostics
  the LSP surfaces
- [DSL quick reference](../reference/dsl-quickref.md) — what every
  keyword the server knows about actually means
- [Roadmap](../roadmap.md) — LSP features planned but not yet built
  (code actions, rename, workspace-level support)

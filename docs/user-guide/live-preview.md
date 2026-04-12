# Live preview

Forge has two commands that keep rendered diagrams in sync with a
`.forge` file as you edit it.

## `forge serve` — browser live reload

Starts an HTTP server, renders all views, and injects a small script
that reloads the page whenever the source file changes.

```bash
forge serve --source architecture.forge --port 4000
```

Open <http://localhost:4000>. Save `architecture.forge` in your editor
and the page refreshes within a fraction of a second.

Useful flags:

- `--style filled` (default) or `--style outline` — two different
  rendering styles. Filled is the colourful version; outline is mono
  line-art suitable for printing or slides.
- `--baseline old.forge` — diff rendering. Elements that are new
  relative to `old.forge` are highlighted in green; changed elements
  amber. Good for reviewing architecture changes in a PR.
- `--present` — presentation mode for animated views. Steps through
  the `animation { … }` frames with keyboard shortcuts.

The server uses Server-Sent Events for reload, so it works in every
modern browser without any client-side build step.

## `forge watch` — CLI rebuild loop

Does the same re-rendering on save but writes SVGs to disk instead of
serving them over HTTP. Use this when you want to open the generated
files in a non-browser viewer (e.g. macOS Preview) or when you're
editing the model on a remote machine.

```bash
forge watch --source architecture.forge --out _site
```

Every save produces fresh files under `_site/diagrams/`. Combine with
an editor preview plugin that auto-reloads images from disk.

## Editor integration

For diagnostics and completion *inside* your editor while you write
the DSL, use the LSP server instead of a watcher:

```bash
forge lsp
```

Editor configs for VS Code, Neovim, Helix, Emacs, IntelliJ, Sublime,
and Zed live in [`forge/EDITORS.md`](../../forge/EDITORS.md).

## Performance

Both `serve` and `watch` re-render every view on every change. For a
model with 30 elements and 10 views, that's usually under 100 ms. For
very large models you can narrow the rebuild to one view at a time:

```bash
forge build --source architecture.forge --view Containers --out _site/diagrams
```

…but this is rarely needed; the full rebuild is fast enough that most
users never hit the limit.

## See also

- [Your first model](first-model.md) — the starter walkthrough uses
  `forge serve`
- [CLI reference](../reference/cli.md) — every `serve` and `watch` flag

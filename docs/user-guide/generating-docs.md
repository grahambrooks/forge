# Generating documentation

`forge generate` turns a `.forge` file into a self-contained static
website — one page per element, one page per view, full-text navigation
between them — ready to host on GitHub Pages, S3, Netlify, or any other
static host.

## The one-liner

```bash
forge generate --source architecture.forge --out _site
open _site/index.html
```

That's it. The `_site` directory contains plain HTML, CSS, and SVG;
no JavaScript build step, no node_modules, no runtime server.

## What you get

```
_site/
├── index.html                  # overview: model name, description, element counts
├── diagrams/
│   ├── context.svg
│   ├── containers.svg
│   └── pipeline.svg
├── elements/
│   ├── api.html                # one page per element
│   ├── db.html
│   └── ...
├── views/
│   ├── context.html            # one page per view
│   ├── containers.html
│   └── ...
├── docs/
│   ├── overview.html           # any `docs { … }` markdown included
│   └── adr-0001.html
├── model.json                  # full JSON export of the model
└── style.css
```

Element pages show the element's description, technology, tags,
properties, the relationships in and out, and which views it appears
in. View pages show the rendered SVG and a legend of every element in
scope.

## Markdown docs

Add a `docs { … }` block to your model to pull in Markdown pages
alongside the generated content:

```forge
docs {
  page "overview" "docs/overview.md" { order 1 }
  page "adr-0001" "docs/adr-0001-rust-for-api.md" { order 2 }
}
```

`forge generate` renders the Markdown with CommonMark, wraps it in the
site template, and adds it to the navigation. Use this for ADRs,
runbooks, onboarding docs — anything that belongs with the architecture
model but doesn't fit the DSL.

## Diff mode

Pass `--baseline` to highlight what changed since a previous commit:

```bash
# Stash the current file, check out main, copy it, come back
git stash
git show main:architecture.forge > /tmp/baseline.forge
git stash pop

forge generate \
  --source architecture.forge \
  --baseline /tmp/baseline.forge \
  --out _site
```

The generated SVGs now highlight added elements in green and modified
elements in amber, so reviewers can see at a glance what the PR
changes. Removed elements are listed on the index page.

## Base URL for GitHub Pages

If you deploy to `https://yourorg.github.io/repo-name/`, tell forge so
internal links resolve correctly:

```bash
forge generate \
  --source architecture.forge \
  --base-url "/repo-name/" \
  --out _site
```

## Rendering style

Two styles, chosen with `--style`:

- `--style filled` (default) — colourful, shaded, suitable for web and
  slides
- `--style outline` — monochrome line-art, suitable for print or dark
  documentation themes

You can run both and serve whichever the user's theme prefers.

## Publishing

For step-by-step deployment recipes see
[`forge/PUBLISHING.md`](../../forge/PUBLISHING.md). The short version:

**GitHub Pages**

```yaml
- uses: actions/configure-pages@v5
- run: forge generate --source architecture.forge --base-url "/${{ github.event.repository.name }}/" --out _site
- uses: actions/upload-pages-artifact@v3
  with:
    path: _site
- uses: actions/deploy-pages@v4
```

**Backstage TechDocs**

Forge ships a `techdocs/mkdocs.yml` template and a flag to produce
MkDocs-compatible Markdown in place of HTML. Point a TechDocs entity
at the output and Backstage renders it inline.

## See also

- [Live preview](live-preview.md) — faster iteration loop while you're
  editing the model; switch to `generate` only when you're ready to
  publish
- [Views](../reference/dsl-quickref.md#views) — every view type the
  generator knows how to render

# Forge documentation

Forge is a unified modelling DSL and toolchain for describing both the
*structure* (C4 architecture) and *processes* (CI/CD pipelines, branching
strategies, deployment topology) of software systems from a single coherent
file, and for keeping that file in sync with a real codebase.

This site is split into three sections. Pick the one that matches what you're
trying to do.

## I want to…

| Goal | Start here |
| --- | --- |
| Install `forge` and run it for the first time | [User guide → Install](user-guide/install.md) |
| Write my first `.forge` model from scratch | [User guide → Your first model](user-guide/first-model.md) |
| Point `forge analyze` at an existing repo and see what it finds | [User guide → Analyzing a codebase](user-guide/analyzing-a-codebase.md) |
| Re-run analyze in CI without losing my hand-authored content | [User guide → Merge mode](user-guide/merge-mode.md) |
| Preview diagrams while I edit the model | [User guide → Live preview](user-guide/live-preview.md) |
| Get hover, completion, and diagnostics in my editor | [User guide → Editor integration (LSP)](user-guide/lsp.md) |
| Lint the model in CI and fail builds on architectural violations | [User guide → Linting](user-guide/linting.md) |
| Publish a static documentation site | [User guide → Generating docs](user-guide/generating-docs.md) |

## Workflows

End-to-end narratives for the three most common ways people use forge.

- [Greenfield: starting a new model from scratch](workflows/greenfield.md)
- [Brownfield: analyzing an existing codebase](workflows/brownfield.md)
- [CI integration: keeping the model in sync](workflows/ci-integration.md)

## Reference

Hard facts pulled straight from the source tree. Use these when you need to
know exactly what a scanner sees, what a CLI flag does, or what syntax the
parser accepts.

- [CLI command reference](reference/cli.md) — every subcommand and flag
- [DSL quick reference](reference/dsl-quickref.md) — the `.forge` syntax in one page
- [Grammar](reference/grammar.md) — formal W3C-style EBNF for the whole DSL
- [Scanners](reference/scanners.md) — what each scanner reads and emits
- [Languages and tools](reference/languages-and-tools.md) — what's supported today
- [Correlations](reference/correlations.md) — the cross-scanner passes
- [Linter rules](reference/linter-rules.md) — the eight built-in architectural checks

## Project status

- [Roadmap](roadmap.md) — prioritized list of remaining features from
  `DESIGN.md`, with per-item design sketches and effort estimates. Read
  this to see what's on deck and what was recently shipped.

## Elsewhere in the repo

- [`DESIGN.md`](../DESIGN.md) — the source-of-truth design spec. Richer and
  more speculative than these docs; worth reading if you're extending forge.
- [`forge/EDITORS.md`](../forge/EDITORS.md) — configuring the LSP for
  VS Code, Neovim, Helix, Emacs, IntelliJ, Sublime, Zed.
- [`forge/PUBLISHING.md`](../forge/PUBLISHING.md) — deploying generated sites
  to GitHub Pages and Backstage TechDocs.
- [`forge/examples/payments.forge`](../forge/examples/payments.forge) — a
  complete reference model with every DSL feature exercised.
- [`forge/examples/ci/analyze.yml`](../forge/examples/ci/analyze.yml) — drop-in
  GitHub Actions workflow for keeping an analyzed model in sync.

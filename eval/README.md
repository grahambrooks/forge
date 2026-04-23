# Forge evaluation framework

A reproducible corpus and two drivers:

- **`run.py`** — baseline: invokes `forge analyze` directly on each project.
- **`run_claude.py`** — companion: invokes Claude Code (headless) with the
  `forge-architect` plugin, which in turn calls `forge_analyze` through the
  MCP server. Measures what a Claude-driven loop produces over the same
  corpus so we can compare model quality, violation counts, and cost.
- **`compare.py`** — diffs the two runs into a side-by-side report.

Both drivers share the clone cache under `work/<name>/` and the corpus in
`corpus.json`, so results are aligned by project id.

The baseline driver clones each project (shallow, pinned ref), runs
`forge analyze`, exports the model, and scores it against per-project
expectations. It emits one JSON result per project plus an aggregated
markdown report.

## Quick start

```bash
# from this directory
cargo build --release --manifest-path ../forge/Cargo.toml

# Baseline — runs forge analyze directly.
./run.py                    # tier 1+2 (13 projects, ~15 min) + static site
./run.py --tier 1           # smoke tests only (5 projects, ~2 min)
./run.py --tier all         # everything, including stretch tier 3
./run.py --only flask,gin   # specific projects
./run.py --no-site          # skip static-site generation
./run.py report             # regenerate report.md from cached results/
./run.py site               # regenerate static site from cached results/
./run.py clean              # delete work/ and results/

# Companion — same corpus, driven by Claude through the forge-architect plugin.
./run_claude.py             # tier 1 by default (API calls cost money)
./run_claude.py --tier 2 --budget 1.00    # broader, $1 per project
./run_claude.py --only flask --model opus # single project, bigger model
./run_claude.py report      # regenerate results-claude/report.md
./run_claude.py clean       # delete results-claude/

# Side-by-side diff.
./compare.py                # writes results-claude/compare.md
```

Results land in `results/`:

- `results/report.md` — aggregate markdown report.
- `results/<name>.json` — structured metrics and checks per project.
- `results/<name>.forge` — the inferred DSL.
- `results/<name>.model.json` — the exported model for drill-down.
- `results/<name>.analyze.log` / `<name>.generate.log` — raw CLI output.
- `results/site/index.html` — corpus-wide landing page with inline SVG
  previews and links into each per-project sub-site.
- `results/site/<name>/` — a full `forge generate` sub-site per project
  (views, element pages, diagrams, `forge.json`). Open any
  `site/<name>/index.html` to navigate the model visually.

Source clones are cached under `work/<name>/`; re-running the driver is
incremental.

## Corpus

`corpus.json` pins 18 projects across three tiers:

| Tier | Purpose | Count | Typical size |
| --- | --- | --- | --- |
| 1 | Fast smoke tests (small libraries & frameworks) | 5 | < 50 MB |
| 2 | Medium complexity (Docker, CI, workspaces) | 8 | 50 – 500 MB |
| 3 | Stretch (large monorepos, k8s-heavy, Helm charts) | 5 | 500 MB+ |

Languages covered: **Rust, Go, Python, TypeScript/JavaScript, Java, Ruby,
PHP**. Build systems: **Cargo, Go modules, pyproject, npm/pnpm workspaces,
Maven, Gradle, Composer, Bundler**. Plus **GitHub Actions**, **Dockerfile /
docker-compose**, **Kubernetes manifests**, **Helm charts**, and **Terraform
/ CloudFormation** for the infra scanners.

Edit `corpus.json` to add or pin projects. Each entry carries an `expect`
block that drives scoring:

```json
{
  "name": "spring-petclinic",
  "repo": "https://github.com/spring-projects/spring-petclinic.git",
  "ref": "main",
  "tier": 2,
  "expect": {
    "min_containers": 1,
    "languages": ["java"],
    "scanners": ["code", "docker", "ci"],
    "max_seconds": 90
  }
}
```

`scanners` lists provenance tags that MUST appear in the output. Forge tags
every inferred element with `inferred:<scanner>`, so this is an end-to-end
check that the scanner actually fired AND produced at least one element.

## What each run checks

For every project the driver records:

- **Input stats** — file count and size of the cloned source tree (excluding
  `.git/`).
- **Output stats** — size of the emitted `.forge` file.
- **Model metrics** — element counts grouped by kind (Container, Component,
  Person, System, Pipeline, Stage, Gate, DeploymentNode, …), relationship
  count, view count, tech-stack entries.
- **Scanner provenance** — count of elements tagged by each of
  `code, semantic, ci, docker, git, k8s, infra`.
- **Timings** — clone seconds, analyze seconds, export seconds.
- **Checks** — pass/fail against `min_containers`, `min_relationships`,
  `min_elements`, each listed scanner, and the `max_seconds` budget.

Status values:

- `pass` — all checks green.
- `fail` — analyzer ran and produced valid output, but at least one
  expectation missed (a regression signal).
- `error` — clone/analyze/export crashed or hit the hard 2× timeout (this is
  a bug, not a corpus-quality miss).

## Layout

```
eval/
├── corpus.json             # project list with refs and expectations
├── _lib.py                 # shared helpers (corpus, clone cache, metrics)
├── run.py                  # baseline driver
├── run_claude.py           # companion driver (Claude + forge-architect)
├── compare.py              # baseline vs Claude diff report
├── README.md               # this file
├── work/                   # git clones (gitignored, regenerable; shared)
├── results/                # baseline output: per-project JSON + .forge + logs + report.md
│   └── site/               # combined static site
│       ├── index.html      # baseline + Claude landing
│       ├── <name>/         # baseline sub-site
│       └── <name>-claude/  # Claude sub-site (only when Claude data exists)
└── results-claude/         # Claude-driven output
    ├── <name>.forge        # .forge file Claude produced
    ├── <name>.model.json   # JSON export of that .forge
    ├── <name>.json         # metrics + usage + checks
    ├── <name>.transcript.json  # raw `claude -p` transcript
    ├── report.md           # per-project aggregate
    └── compare.md          # side-by-side vs baseline
```

## Static site for visual inspection

Both drivers feed a single site generator (`sitegen.py`) that produces a
combined landing page at `results/site/index.html` with:

- A pill summary of baseline + Claude run totals (pass/fail/error/$ spend).
- A grouped summary table: for every project that appears in either run,
  columns for Baseline (status / elements / views / time) and Claude
  (status / elements / views / time / cost), plus a Δ-elements column.
- A cards grid: two inline SVG previews per project when both runs have
  produced a sub-site — one rendered from the baseline `.forge`, one from
  Claude's — so you can eyeball differences at a glance.
- Links into per-project sub-sites: `/<name>/` (baseline) and
  `/<name>-claude/` (Claude), each a full `forge generate` output.

`forge analyze` emits elements but no views, so the driver synthesizes a
small default `views { ... }` block based on what the model actually
contains (a `container-view` if Containers exist, a `pipeline-view` if
Pipelines exist, and so on). The augmented `.forge` is written alongside
as `<name>.viewed.forge` and fed to `forge generate`.

Regenerate the site without re-running the analyzers:

```bash
./sitegen.py                    # rebuilds everything (forge generate × N)
./sitegen.py --skip-regenerate  # rewrites only index.html; fast iteration
```

`./run.py` and `./run_claude.py` both call `sitegen.build_site()` at the
end of a run (pass `--no-site` to skip).

## Companion driver (`run_claude.py`)

The companion runs the **same corpus** through Claude Code in headless mode
(`claude -p`) with the `forge-architect` plugin — activated inline with
`--plugin-dir ../integrations/claude-plugin/forge-architect` so no prior
install is required. Claude's prompt invokes the `model-repository` skill,
which steers it to call `forge_analyze` via the MCP, review with
`forge_check`, and save the final `.forge`.

### Prerequisites

- `forge` binary on PATH (same as baseline).
- `claude` CLI on PATH (`claude --version` should work).
- A signed-in Claude session *or* `ANTHROPIC_API_KEY` in the environment.

### What it measures

Per project:

- **Model shape** — identical metrics to the baseline (element counts by
  kind, relationships, views, provenance tags) so the numbers align.
- **Violations** — `forge check` at `severity: info` run on Claude's output.
- **Usage** — input/output tokens, cache tokens, turns, and API cost (USD).
- **Wall clock** — clone + Claude loop + export, separately timed.
- **Claude's summary** — the last assistant message from the transcript,
  preserved verbatim so a human can scan what Claude thought.

### Budget control

Every project is capped at `--budget` (default $0.50) via Claude's
`--max-budget-usd`. A wall-clock `--timeout` (default 600s) is a belt-and-
braces second line. The driver defaults to `--tier 1` because API spend
adds up fast; opt into tier 2/3 deliberately.

### Comparing the runs

`./compare.py` pairs `results/<name>.json` with `results-claude/<name>.json`
and emits three tables:

1. **Side by side** — status, element count, relation count, view count,
   wall clock, and Claude cost for every project that appears in both runs.
2. **Provenance deltas** — which scanner tags are present in one run but
   not the other. Normally identical, because both runs invoke the same
   analyzer; divergence flags something interesting (e.g. Claude truncated
   the scan, or the MCP surfaced an extra correlation).
3. **Divergent projects** — full detail only for projects where the runs
   disagreed on pass/fail or element count.

### When the numbers should diverge

The expected result is **close to identical** headline metrics — the Claude
loop's first move is to call `forge_analyze`, which runs the same analyzer.
Divergence signals one of:

- Claude hit its budget / timeout before `forge_analyze` returned.
- Claude passed a narrower `scanners` list (the skill suggests this for
  small libraries — compare the provenance delta column).
- Claude edited the `.forge` post-analyze to fix a violation the analyzer
  left. Today the prompt explicitly tells Claude *not* to edit — so this
  should be rare. Toggle the prompt in `run_claude.py` if you want to
  measure the edit behaviour instead.

## CI integration

A tier-1 run is a sensible CI gate — under 2 minutes end-to-end, hits all
core scanners. Example invocation:

```bash
./eval/run.py --tier 1 --forge ./forge/target/release/forge
test $? -eq 0  # hard errors fail the build
grep -q "status.*fail" eval/results/*.json && echo "soft regressions detected"
```

## Extending

- **Add a project**: append to `corpus.json` with a pinned `ref` and realistic
  expectations. Start with loose expectations, tighten after a baseline run.
- **Tune a check**: edit the `expect` block in `corpus.json`. Don't tune
  checks against the binary in `run.py` — the corpus is the contract.
- **Add a metric**: extend `Metrics` and `compute_metrics()` in `_lib.py`.
  Both drivers will pick it up; surface it in each driver's `write_report()`
  and in `compare.py` as needed.
- **Change Claude's prompt**: `PROMPT_TEMPLATE` in `run_claude.py`. The
  current prompt deliberately forbids hand edits so the comparison measures
  analyzer + MCP output. Loosen it to measure what Claude adds when allowed
  to edit.

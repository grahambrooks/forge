# Roadmap

A prioritized list of features from [`DESIGN.md`](../DESIGN.md) that
are not yet implemented, with enough design detail per item to serve
as a starting point for whoever picks the work up.

## What this document is (and isn't)

**Is:** the current backlog, sorted by priority. Each item has a
motivation grounded in `DESIGN.md`, a concrete design sketch, the
files likely to change, an API/DSL snippet where relevant, effort
estimate, and acceptance criteria.

**Isn't:** the `DESIGN.md` spec itself. `DESIGN.md` describes the
whole product vision; this document tracks the delta between that
vision and `main`. When the two disagree, this document is the more
recent one. Changes to user-visible semantics should still land in
`DESIGN.md` first, then this file gets updated.

**Isn't:** a commitment or a schedule. Nothing here has a deadline,
owner, or guaranteed release. It's a backlog ordered by value.

## Recently shipped

Work that has landed on `main` since `DESIGN.md` §10's "Remaining"
section was last updated. Several items the spec still lists as
remaining are actually done; these are called out explicitly below
so nobody picks them up twice.

### Commits on `main` from the current analyze-pipeline work

| Commit | What shipped |
| --- | --- |
| [`89aa76b`](../forge/src/analyze/code.rs) | Symgraph-backed analyze pipeline: real manifest parsing via `symgraph::extraction::manifest`, path-prefix `ContainerIndex` replacing slug-contains attribution, Cargo/npm/pnpm/yarn workspace expansion, framework tech inference from the extracted dep set |
| [`73bca38`](../forge/src/analyze/merge.rs) | `analyze --merge` for CI-safe re-runs; provenance via `inferred` tags; user content preserved on every re-run |
| [`7069a15`](../forge/src/analyze/correlate.rs) | Env-var exact-match correlation linking source-code consumers to docker-compose providers via `forge:env_reads` / `forge:env_provides` |
| [`3ce0e6c`](../forge/src/analyze/git.rs) | CODEOWNERS → team ownership with last-rule-wins semantics; owner name normalisation; contributor fallback when no CODEOWNERS file exists |
| [`2ec1a43`](../forge/examples/ci/analyze.yml) | K8s env providers for Deployment / StatefulSet / DaemonSet (direct `env:`, `valueFrom`, `envFrom`); reference CI workflow template |
| [`610ab3c`](../forge/src/analyze/correlate.rs) | Connection-string fallback (DATABASE_URL → postgres-tagged container) and pipeline-env correlation (CI stage → synthetic Environment → k8s namespace deployment) |
| _pending_ | [Item 7](#7-data-classification-tags--visual-indicators): `dataClass` DSL keyword, shield badges on containers (pii/financial/secret/public/internal colouring), `data-class-boundary` linter rule |
| _pending_ | [Item 4](#4-dynamic-view-type--numbered-relationship-ordering): `dynamic` view type with `<num>. src -> dst "label"` ordered relationships, circled step badges, auto-generated animation frames |
| _pending_ | [Item 5](#5-composite-view-type--grid-of-views): `composite` view type, `grid N M` + `cell "key"` DSL, child SVGs assembled as nested `<svg>` elements with per-cell frames and captions |
| _pending_ | **DSL v2 consistency overhaul** ([grammar.md](reference/grammar.md)): every keyword kebab-case, every view kind gets the `-view` suffix, every process element uses `id = kind "Name"` bindings, single-spelling keywords, structured endpoint method/path, strict parse errors on unknown keywords, view bodies are optional, `styles`/`produces` removed. Breaking change with no backwards compatibility. |

### Features `DESIGN.md §10` still lists as "Remaining" but are actually done

- **`forge mcp`** — Model Context Protocol server over stdio, exposing
  six tools (`forge_query`, `forge_render`, `forge_check`,
  `forge_element_detail`, `forge_search`, `forge_validate`). Source:
  [`forge/src/mcp.rs`](../forge/src/mcp.rs).
- **`forge export`** — standalone JSON/YAML export command. Source:
  [`forge/src/main.rs`](../forge/src/main.rs) `cmd_export`.
- **`forge import`** — PlantUML C4 and Mermaid flowchart import to
  `.forge`. Source: [`forge/src/main.rs`](../forge/src/main.rs)
  `cmd_import`.
- **SARIF output** — `forge check --format sarif` produces SARIF
  2.1.0 for GitHub Code Scanning. Source:
  [`forge/src/check.rs`](../forge/src/check.rs).
- **Git scanner** — via `gix`, infers branching strategy, contributor
  stats, and CODEOWNERS-based team attribution. Source:
  [`forge/src/analyze/git.rs`](../forge/src/analyze/git.rs).
- **Kubernetes scanner** — Deployment / StatefulSet / DaemonSet /
  Service / Ingress / ConfigMap. Source:
  [`forge/src/analyze/k8s.rs`](../forge/src/analyze/k8s.rs).
- **OpenAPI spec parsing** — Lives in the `infra` scanner alongside
  CloudFormation and Terraform. Source:
  [`forge/src/analyze/infra.rs`](../forge/src/analyze/infra.rs)
  `parse_openapi`.
- **Tree-sitter AST analysis** — Delegated to `symgraph` rather than
  implemented in-tree. Source:
  [`forge/src/analyze/semantic.rs`](../forge/src/analyze/semantic.rs).

---

## Prioritization framework

Four tiers, strict ordering within a tier is not implied.

- **P0 — foundational.** Unblocks other work or closes an obvious
  functional gap every user hits. Do these first.
- **P1 — high user value.** Meaningful features that materially
  improve what forge does for users today.
- **P2 — extends existing capability.** Nice to have. Each one adds
  polish to a feature that already works.
- **P3 — expensive or speculative.** Large rewrites, uncertain
  payoff, or operational-only work with no user-visible feature.

Effort estimates:

- **S** — ~1 day of focused work
- **M** — 2-5 days
- **L** — 1+ weeks

---

## Priority index

| # | Priority | Effort | Item | One-line summary |
| --- | --- | --- | --- | --- |
| [1](#1-png--pdf-export-via-resvg) | P0 | M | PNG / PDF export via `resvg` | Raster output for PRs, slides, printed docs |
| [2](#2-http-transport-for-forge-mcp) | P0 | S | HTTP transport for `forge mcp` | Unblocks remote MCP clients |
| [3](#3-force-directed-layout--landscape-view-type) | P0 | L | Force-directed layout + `landscape` view | Enterprise-scale overview views |
| ~~[4](#4-dynamic-view-type--numbered-relationship-ordering)~~ | ~~P1~~ | ~~M~~ | ~~`dynamic` view type~~ | **Shipped** — see [Recently shipped](#recently-shipped) |
| ~~[5](#5-composite-view-type--grid-of-views)~~ | ~~P1~~ | ~~M~~ | ~~`composite` view type~~ | **Shipped** — see [Recently shipped](#recently-shipped) |
| [6](#6-write-capable-mcp-tools) | P1 | M | Write-capable MCP tools | `forge_analyze`, `forge_diff`, `forge_suggest_fix` |
| ~~[7](#7-data-classification-tags--visual-indicators)~~ | ~~P1~~ | ~~S~~ | ~~Data classification tags~~ | **Shipped** — see [Recently shipped](#recently-shipped) |
| [8](#8-flowview--runbook-element-kind) | P1 | L | `flowView` + `runbook` kind | Runbooks as first-class model content |
| [9](#9-smil-animation-output-mode) | P2 | S | SMIL animation mode | JS-free animated SVG output |
| [10](#10-extends--override-workspace-inheritance) | P2 | S | `!extends` / `!override` directives | Workspace inheritance for per-env overrides |
| [11](#11-include-url-remote-includes) | P2 | S | `!include <url>` | Centralised shared fragments |
| [12](#12-client-side-search-index) | P2 | S | Client-side search index | In-site search for generated docs |
| [13](#13-gif--webm-animation-export) | P3 | L | GIF / WebM animation export | Embed animations in JS-free surfaces |
| [14](#14-performance-optimization-for-10k-element-models) | P3 | L | 10k-element performance | Profile + fix quadratic hotspots |
| [15](#15-cross-compilation--release-automation) | P3 | M | Cross-compilation + release binaries | Linux / macOS / Windows artefacts |
| [16](#16-cargo-workspace-restructure) | P3 | M | Cargo workspace restructure | Split into forge-core / forge-analyze / forge-render |

---

## P0 items

### 1. PNG / PDF export via resvg

**Priority:** P0 &nbsp; **Effort:** M &nbsp; **Depends on:** none

**Problem.** Every `forge build` and `forge generate` output is SVG.
That's fine for web and modern editors, but PRs (GitHub's diff viewer
rasterises SVGs with fuzzy anti-aliasing), slide decks, PDF reports,
and printed documentation need raster formats. `DESIGN.md §11` already
names `resvg` as the intended implementation — it just hasn't been
wired up yet.

**Design.**

- Add a `--format` flag to `forge build` accepting `svg` (default),
  `png`, and `pdf`.
- Plumb an optional `--raster <png|pdf>` flag through `forge generate`
  so every diagram in the static site can be emitted as an alternate
  asset alongside the SVG (site HTML continues to reference the SVG;
  the raster files are available for download).
- Use `resvg` + `tiny-skia` for PNG. Use `svg2pdf` (a companion crate
  published by the resvg author) for PDF.
- DPI / scaling controlled by `--scale <N>` (default `2.0` for
  retina-quality PNGs).

**Files to touch.**

- [`forge/Cargo.toml`](../forge/Cargo.toml) — add `resvg`, `tiny-skia`,
  and `svg2pdf`. All three are pure-Rust and compatible with the
  existing dep tree.
- [`forge/src/main.rs`](../forge/src/main.rs) — `--format` and
  `--scale` flags on the `Build` command, passed through to the
  build dispatcher.
- [`forge/src/render.rs`](../forge/src/render.rs) — new
  `render_to_format(svg, format, scale)` entry point alongside the
  existing `render_svg`.
- [`forge/src/generate.rs`](../forge/src/generate.rs) — optional
  side-channel asset generation path when the raster flag is set.

**API sketch.**

```rust
// forge/src/render.rs

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Svg,
    Png,
    Pdf,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
        }
    }
}

pub fn render_to_format(
    svg: &str,
    format: OutputFormat,
    scale: f32,
) -> Result<Vec<u8>, RasterError> {
    match format {
        OutputFormat::Svg => Ok(svg.as_bytes().to_vec()),
        OutputFormat::Png => render_png(svg, scale),
        OutputFormat::Pdf => render_pdf(svg, scale),
    }
}
```

User-facing CLI:

```bash
forge build --format png --scale 3 --out out/
forge build --format pdf --out out/
forge generate --source architecture.forge --raster png --out _site
```

**Verification.**

- Unit tests on `render_to_format` for a minimal hand-crafted SVG,
  asserting the returned bytes start with the PNG (`89 50 4E 47`) or
  PDF (`%PDF`) magic.
- Integration test in `render::tests` that runs every view from
  `examples/payments.forge` through the new path and asserts the
  resulting buffers are non-empty and carry the right magic bytes.
- Size regression check: PNG output for a sample view should be
  under some reasonable cap (say, 500 KB at `--scale 2`).

**Open questions.**

- Which font does `resvg` use for text measurement? The existing
  layout pipeline uses `ab_glyph`. Need to verify the two agree on
  glyph widths, or the raster and SVG will disagree on box sizing.
  Worst case, ship a bundled TTF and force resvg to use it via its
  font loader API.
- Do we want a matching `--background <color|transparent>` flag?
  First cut: always transparent, match the SVG output.

---

### 2. HTTP transport for `forge mcp`

**Priority:** P0 &nbsp; **Effort:** S &nbsp; **Depends on:** none

**Problem.** `forge mcp` only speaks stdio today. That works for
Claude Code running it as a subprocess, but it blocks every
multi-client or remote scenario: a browser-based Cursor session
talking to a forge-mcp running elsewhere, a Slack bot that wants to
query a repo's architecture on demand, or a central orchestrator
running one forge-mcp per repo. `DESIGN.md §8.6` explicitly calls
for both stdio *and* HTTP transports.

**Design.**

- Add `--transport <stdio|http>` and `--port <N>` flags to
  `forge mcp`. Default remains `stdio` so the existing Claude Code
  config works unchanged.
- HTTP transport exposes a single `POST /rpc` endpoint accepting MCP
  request bodies in the same JSON-RPC framing as stdio. Streaming
  responses for long-running tool calls use Server-Sent Events on
  `GET /rpc/stream/<request-id>`.
- Refactor the handler in `mcp.rs` to separate transport (read a
  request, write a response) from the tool dispatcher (look up the
  tool name, call the handler). Only the transport layer changes;
  every existing tool continues to work.

**Files to touch.**

- [`forge/src/mcp.rs`](../forge/src/mcp.rs) — extract a `McpTransport`
  trait; split current handler into a `StdioTransport` impl; add an
  `HttpTransport` impl using `axum` (already transitively pulled in
  via `symgraph`).
- [`forge/src/main.rs`](../forge/src/main.rs) — new `--transport` and
  `--port` flags on the `Mcp` command.

**API sketch.**

```rust
#[async_trait]
pub trait McpTransport {
    async fn run(self, handler: McpHandler) -> Result<(), McpError>;
}

pub struct StdioTransport;
pub struct HttpTransport {
    pub port: u16,
    pub bind: IpAddr, // default 127.0.0.1
}

// main.rs dispatch
match transport {
    "stdio" => StdioTransport.run(handler).await?,
    "http" => HttpTransport::new(port).run(handler).await?,
    _ => die("unknown transport"),
}
```

CLI:

```bash
forge mcp --source architecture.forge                             # stdio
forge mcp --source architecture.forge --transport http --port 4100
```

**Verification.**

- Unit test in `mcp::tests` that spawns `HttpTransport` on an
  ephemeral port and sends a hand-crafted JSON-RPC request for
  `forge_query`. Asserts a well-formed MCP response.
- Backwards compatibility: the existing stdio-mode integration tests
  continue to pass unchanged.

**Open questions.**

- Authentication. Default is localhost-only with no auth (same
  threat model as a local dev server). Add `--bind 0.0.0.0` with a
  loud stderr warning when used, and document that production
  deployments should put an authenticating reverse proxy in front.
- SSE streaming is only needed if any tool becomes long-running.
  Current six tools are synchronous, so streaming can ship in a
  follow-up; the first version supports request/response only.

---

### 3. Force-directed layout + `landscape` view type

**Priority:** P0 &nbsp; **Effort:** L &nbsp; **Depends on:** none

**Problem.** Forge has 12 layered / lane / grid layout algorithms but
no force-directed algorithm. The `landscape` view type from
`DESIGN.md §9` — "all systems in the enterprise" — needs
force-directed layout to be legible at 100+ nodes. Without it, any
enterprise-wide view forces users to hand-arrange elements or fall
back to another tool.

**Design.**

- New `ViewKind::Landscape` in `model.rs`.
- New parser rule accepting `landscape "Key" { ... }` inside the
  `views` block.
- New `layout_landscape` in `layout.rs` implementing a custom
  Barnes-Hut force-directed algorithm (`DESIGN.md §11` names this
  as the intended implementation — it's the one layout algorithm
  the project committed to writing from scratch).
- The view renders every top-level `system` element, with
  relationships between systems. Intra-system detail is elided
  (systems are drawn as icons with a name label; their contained
  containers are hidden at this zoom level).
- Cross-system edges are rendered as bezier curves rather than the
  orthogonal routes used elsewhere — orthogonal routing looks
  terrible on force-directed graphs.
- New `forge-landscape` CSS class on the root `<svg>` for styling.

**Files to touch.**

- [`forge/src/model.rs`](../forge/src/model.rs) — add
  `ViewKind::Landscape`.
- [`forge/src/parser.rs`](../forge/src/parser.rs) — accept
  `landscape` keyword in the views block.
- [`forge/src/layout.rs`](../forge/src/layout.rs) — new
  `layout_landscape` function + `ForceDirectedSim` helper type
  implementing Barnes-Hut.
- [`forge/src/render.rs`](../forge/src/render.rs) — new edge routing
  path for bezier curves on force-directed layouts; new node style
  for system icons.

**API sketch.**

```rust
// forge/src/layout.rs

fn layout_landscape(
    model: &Model,
    view: &View,
    tm: &TextMeasurer,
) -> Layout {
    let nodes = collect_systems(model, view);
    let edges = collect_inter_system_edges(model, &nodes);

    let sim = ForceDirectedSim::new(&nodes, &edges)
        .with_iterations(300)
        .with_theta(0.9)       // Barnes-Hut approximation threshold
        .with_repulsion(400.0)
        .with_spring_length(180.0);

    sim.run_to_stable()
}

struct ForceDirectedSim<'a> {
    nodes: Vec<NodePosition>,
    edges: &'a [Edge],
    theta: f32,
    // ... tuning parameters
}

impl<'a> ForceDirectedSim<'a> {
    fn run_to_stable(mut self) -> Layout {
        for _ in 0..self.iterations {
            let quadtree = self.build_quadtree();
            self.apply_repulsion(&quadtree);
            self.apply_spring_forces();
            self.integrate();
            if self.converged() { break; }
        }
        self.into_layout()
    }
}
```

DSL:

```forge
views {
  landscape "Enterprise" {
    include *
    title "Enterprise Landscape"
  }
}
```

**Verification.**

- Fixture under `forge/tests/fixtures/render/landscape/` with 3
  systems and 4 inter-system relationships. Integration test asserts
  the emitted layout has all 3 nodes, all 4 edges, and node positions
  are pairwise distant enough that labels don't overlap — verify via
  `ab_glyph` text measurement of each node's label.
- Regression: a 50-system fixture to ensure the Barnes-Hut
  implementation actually handles scale. Assert total runtime under
  2 seconds.
- Visual sanity: render the fixture to SVG and eyeball once during
  review; snapshot the emitted SVG structure (not pixel output) so
  layout refactors don't silently break it.

**Open questions.**

- Should the landscape view also render a faint "ghost" of containers
  inside each system for context, or keep each system purely iconic?
  First cut: iconic only. Add a `--detail` toggle later if users ask.
- How much Barnes-Hut tuning should be DSL-exposed? Keep parameters
  internal for now; expose `forceSpringLength` and `forceRepulsion`
  as view-level overrides if fixture testing shows the defaults are
  wrong for common models.

---

## P1 items

### 4. `dynamic` view type — numbered relationship ordering

> **Status: shipped.** `dynamic scope "Key" { … }` blocks accept
> `<num>. src -> dst "label"` ordered relationships, circled step
> badges render on each edge, and dynamic views with no explicit
> animation auto-generate one frame per step. The design notes below
> are retained for historical reference.

**Priority:** P1 &nbsp; **Effort:** M &nbsp; **Depends on:** none

**Problem.** `DESIGN.md §9` lists `dynamic` as "animated sequence
(numbered relationship ordering)". It's the view that explains *how*
a request flows through a system — the classic C4 dynamic view that
Structurizr popularised. Forge has a frame-based animation engine
but no way to express "arrow A fires first, then arrow B, then arrow
C" as a static property of a view. Users who want sequence semantics
today have to hand-author the equivalent via `animation { frames }`,
which is verbose.

**Design.**

- New `ViewKind::Dynamic` in `model.rs`.
- New `order: Option<u32>` field on `Relationship`. `None` means
  "unordered" (current behaviour); `Some(n)` means "this is step `n`
  in a numbered sequence."
- Inside a `dynamic` view block, relationships can carry a leading
  number: `1. web -> api "login request"`. The parser treats the
  leading number as setting the relationship's `order`.
- Renderer draws every ordered arrow with a circled step number near
  the arrow midpoint (reuse the gate-diamond rendering code for the
  circle).
- A `dynamic` view without an explicit `animation { frames }` block
  auto-generates one frame per numbered step, so `forge serve
  --present` steps through the sequence naturally.

**Files to touch.**

- [`forge/src/model.rs`](../forge/src/model.rs) — add
  `order: Option<u32>` to `Relationship`; add `ViewKind::Dynamic`.
- [`forge/src/parser.rs`](../forge/src/parser.rs) — accept
  `<num>. source -> target "label"` inside `dynamic { ... }` blocks.
  Parser must not accept leading numbers in other block types (to
  keep the syntax unambiguous).
- [`forge/src/layout.rs`](../forge/src/layout.rs) — new
  `layout_dynamic` (thin wrapper over container layout; pass the
  ordering metadata through to the renderer).
- [`forge/src/render.rs`](../forge/src/render.rs) — step-number
  badge rendering on edges.
- [`forge/src/animate.rs`](../forge/src/animate.rs) — auto-generate
  one frame per ordered step when no explicit animation is declared
  on a dynamic view.

**API sketch.**

```rust
// forge/src/model.rs

pub struct Relationship {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
    pub order: Option<u32>,   // new
}
```

DSL:

```forge
views {
  dynamic app "LoginFlow" {
    title "User Login Flow"
    1. customer -> web "submits credentials" "HTTPS"
    2. web -> api "POST /login"
    3. api -> db "SELECT user by email"
    4. api -> web "JWT + session cookie"
    5. web -> customer "dashboard"
  }
}
```

**Verification.**

- Parser test roundtrips a 5-step dynamic view through parse → emit
  → re-parse; assert `order` survives.
- Layout test asserts every step badge appears in the rendered SVG
  at the expected arrow midpoint coordinates.
- Animation test asserts that a dynamic view with no explicit
  animation block produces `N` auto-generated frames matching the
  step count.

**Open questions.**

- How do ordered arrows interact with an *explicit* `animation {
  frames }` block on the same dynamic view? Rule: explicit frames
  override the auto-generated ones. Document this in the DSL
  reference.
- What happens if the user numbers steps non-contiguously (1, 2, 5)?
  Treat gaps as acceptable; animation steps through `order` values
  in sorted order, not by index.

---

### 5. `composite` view type — grid of views

> **Status: shipped.** `composite "Key" { grid N M; cell "view-key"; … }`
> blocks dispatch to each cell view's normal layout/render pipeline
> and assemble the child SVGs into a row-major grid, with per-cell
> frames and captions. Nested composites are short-circuited to
> avoid infinite recursion. The design notes below are retained for
> historical reference.

**Priority:** P1 &nbsp; **Effort:** M &nbsp; **Depends on:** none

**Problem.** Executive dashboards and architecture review slides
want a single image that shows four or six views side by side.
Today users export separate SVGs and arrange them in a slide or a
wiki. `DESIGN.md §9` calls for a `composite` view that does this
natively so the "full picture" lives in one file that stays in sync
with the model.

**Design.**

- New `ViewKind::Composite` in `model.rs`, backed by a
  `CompositeView` side struct holding a grid layout and ordered cell
  references.
- DSL accepts a `composite "Key" { grid N M; cell <view-key>; ... }`
  block. Row-major order: the first N cells form the top row.
- Renderer dispatches to the normal layout/render pipeline for each
  referenced view, then assembles the resulting SVGs as nested
  `<svg x y width height>` elements inside a parent `<svg>` with
  clipped viewBoxes.

**Files to touch.**

- [`forge/src/model.rs`](../forge/src/model.rs) — add
  `ViewKind::Composite` plus a `CompositeView` struct alongside the
  existing `Animation` side struct.
- [`forge/src/parser.rs`](../forge/src/parser.rs) — new block
  parser for `composite`.
- [`forge/src/layout.rs`](../forge/src/layout.rs) — `layout_composite`
  dispatches to each cell's view, collects the resulting layouts,
  then assigns grid-relative offsets.
- [`forge/src/render.rs`](../forge/src/render.rs) — new
  `render_composite` that nests child SVGs with the correct
  `x`/`y`/`width`/`height`.

**API sketch.**

```rust
// forge/src/model.rs

pub struct CompositeView {
    pub cells: Vec<String>,         // view keys in row-major order
    pub cols: u32,
    pub rows: u32,
    pub cell_size: (u32, u32),      // pixel dims per cell
    pub title: Option<String>,
}
```

DSL:

```forge
views {
  composite "Dashboard" {
    grid 2 2
    cell "Context"
    cell "Containers"
    cell "Pipeline"
    cell "Deployment"
    title "System Overview"
  }
}
```

**Verification.**

- Fixture referencing four existing views in a 2×2 grid. Test
  asserts the emitted SVG root contains four child `<svg>` elements
  with correct `x`/`y`/`width`/`height` attributes.
- Each child carries a `data-view="<key>"` attribute so downstream
  tooling can identify which cell is which.
- Regression: a composite that references a non-existent view key
  produces a parser-level error pointing at the offending `cell`
  line.

**Open questions.**

- Cell auto-scaling vs fixed cell size? First cut: fixed cell size
  with sensible defaults (`600×400`). Add `--auto-fit` semantics
  later if needed.
- Inter-cell gap / border? Default 20 px gap; override via
  `gap <n>` inside the composite block.

---

### 6. Write-capable MCP tools

**Priority:** P1 &nbsp; **Effort:** M &nbsp; **Depends on:**
[item 2](#2-http-transport-for-forge-mcp) (helpful but not required)

**Problem.** Forge's MCP server exposes six read-only tools
(`forge_query`, `forge_render`, `forge_check`,
`forge_element_detail`, `forge_search`, `forge_validate`). An agent
can describe what's in the model but can't update it. The most
valuable agent workflow — "analyze the repo and keep the model in
sync with the code" — isn't possible without write tools.

**Design.** Three new MCP tools.

- **`forge_analyze`.** Runs `analyze::analyze()` over a configured
  path and returns a JSON summary of what changed. Honours `--merge`
  semantics when an existing `.forge` file is referenced. Takes
  `{ path, merge?, scanners? }` args. Returns `{ elements_added,
  elements_removed, relationships_added, diff_summary }`.
- **`forge_diff`.** Runs the existing `diff.rs` module against a
  baseline `.forge` file and returns added / modified / removed
  elements in structured form. Takes `{ baseline, current }` args;
  returns the `DiffResult` structure from `diff.rs` serialised as
  JSON.
- **`forge_suggest_fix`.** Takes a lint-violation identifier from a
  previous `forge_check` call and returns a proposed `.forge`
  snippet that would resolve it. Template-driven: each of the eight
  built-in rules maps to a canned fix template (for example,
  `missing-descriptions` returns an edit that inserts a
  `description "TODO"` line at the offending element's position).

**Files to touch.**

- [`forge/src/mcp.rs`](../forge/src/mcp.rs) — three new `tool_*`
  functions and register them in the `tools/list` response.
- [`forge/src/diff.rs`](../forge/src/diff.rs) — add a
  `serde::Serialize` derive to `DiffResult` and its subfields if
  they don't already have one.
- [`forge/src/analyze/mod.rs`](../forge/src/analyze/mod.rs) —
  already returns a `Model`; no change required.

**API sketch.**

```rust
// forge/src/mcp.rs

fn tool_analyze(&self, args: &Value) -> String {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let merge_into: Option<PathBuf> = args
        .get("merge")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    // ... run analyze::analyze() ...
    serde_json::to_string(&AnalyzeResult { /* counts */ }).unwrap()
}

fn tool_diff(&self, args: &Value) -> String { /* ... */ }
fn tool_suggest_fix(&self, args: &Value) -> String { /* ... */ }
```

**Verification.**

- Unit tests for each tool that pass hand-crafted JSON args and
  assert the response parses as valid MCP.
- Integration test that runs a scripted MCP session against the
  `payments.forge` example: call `forge_check`, pick a violation,
  call `forge_suggest_fix` on it, assert the returned snippet is
  well-formed forge DSL.

**Open questions.**

- Should `forge_suggest_fix` return a unified-diff patch or a
  structured edit? Plan: structured edit
  (`{ file, range, replacement }`) — clients can convert to any
  patch format they need.
- Does `forge_analyze` write to disk, or return the model without
  writing? Plan: return-only by default; accept an optional
  `write: true` that writes to the merge target. Safer default is
  no side effects.

---

### 7. Data classification tags + visual indicators

> **Status: shipped.** `dataClass` DSL keyword, shield badge
> rendering, and the `data-class-boundary` linter rule are on `main`.
> The design notes below are retained for historical reference.

**Priority:** P1 &nbsp; **Effort:** S &nbsp; **Depends on:** none

**Problem.** `DESIGN.md §10` Phase 6 lists "Data classification:
PII, financial, public tags per data store with visual indicators."
Trust boundaries already exist and do something similar at the zone
level, but per-container data classification is a different
dimension. One container can hold PII *and* financial data,
distinct from which zone it lives in.

**Design.**

- Add a `dataClass` DSL keyword to container declarations that
  accepts one or more classification levels:
  `dataClass "pii" "financial"`.
- Store in a new `data_classes: Vec<String>` field on `Element`.
- Renderer adds small coloured shield badges to any container whose
  `data_classes` is non-empty. Colours: `pii` = purple,
  `financial` = gold, `public` = green, `secret` = red, `internal`
  = grey. Unrecognised values also render as grey so the system is
  extensible.
- New built-in linter rule `data-class-boundary` warns when a
  container with a `pii` data class is reachable from a `person`
  without passing through at least one container tagged
  `encryption` or `gateway`.

**Files to touch.**

- [`forge/src/model.rs`](../forge/src/model.rs) — add
  `data_classes: Vec<String>` to `Element`.
- [`forge/src/parser.rs`](../forge/src/parser.rs) — accept
  `dataClass "level" "level2" ...` inside container blocks.
- [`forge/src/analyze/emit.rs`](../forge/src/analyze/emit.rs) — emit
  the field on round-trip.
- [`forge/src/render.rs`](../forge/src/render.rs) — shield badge
  rendering.
- [`forge/src/check.rs`](../forge/src/check.rs) — new rule.

**API sketch.** DSL:

```forge
db = container "Ledger DB" {
  technology "PostgreSQL 16"
  tags "database"
  dataClass "pii" "financial"
}
```

Model:

```rust
pub struct Element {
    // ... existing fields ...
    pub data_classes: Vec<String>,
}
```

**Verification.**

- Parser test: round-trip a container with two data classes.
- Render test: assert the emitted SVG contains both a purple PII
  shield and a gold financial shield, positioned at the container's
  top-right.
- Lint test: fixture with `customer -> api -> db` where `db` has
  `dataClass "pii"` and `api` has no `gateway` tag; assert the new
  `data-class-boundary` rule fires.

**Open questions.**

- Fixed enum or free-form tags? Plan: curated set
  (`pii`, `financial`, `secret`, `public`, `internal`) known to the
  renderer, but accept anything — unrecognised values render as the
  grey fallback badge so teams can extend without patching forge.

---

### 8. `flowView` + `runbook` element kind

**Priority:** P1 &nbsp; **Effort:** L &nbsp; **Depends on:** none

**Problem.** `DESIGN.md §9` lists `flowView` as "sequence of
operations in a runbook." `DESIGN.md §10` Phase 6 mentions "on-call
/ runbook links: operational context per container linking to
external runbook systems." Today forge has no way to model a
runbook at all — runbooks live in Confluence or PagerDuty,
disconnected from the architecture model.

**Design.** Two coordinated additions.

1. New top-level `runbook "Key" { ... }` block (peer of `process`
   and `deployment`). Each runbook contains metadata and an ordered
   list of `step "description" { ... }` items with optional
   `command`, `check`, and `rollback` sub-fields.
2. New `flowView` view type renders a runbook as a vertical
   flowchart. Steps are boxes; `check` sub-fields render as diamond
   decision nodes in the style of existing gates.

Runbooks can link back to the containers they operate on via
`targets api db`.

**Files to touch.**

- [`forge/src/model.rs`](../forge/src/model.rs) — new `Runbook` and
  `RunbookStep` structs; new `runbooks: Vec<Runbook>` field on
  `Model`; new `ViewKind::FlowView`.
- [`forge/src/parser.rs`](../forge/src/parser.rs) — two new block
  parsers (`runbook` and the `flowView` entry inside `views`).
- [`forge/src/layout.rs`](../forge/src/layout.rs) — new
  `layout_flow_view`. Reuse the stage layout geometry — flowViews
  are essentially linear pipelines with decision nodes.
- [`forge/src/render.rs`](../forge/src/render.rs) — runbook-specific
  node shapes (reuse gate-diamond code for checks).
- [`forge/src/analyze/emit.rs`](../forge/src/analyze/emit.rs) —
  round-trip the new blocks.

**API sketch.**

```rust
// forge/src/model.rs

#[derive(Debug, Clone)]
pub struct Runbook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,        // optional external link (PagerDuty, Confluence)
    pub targets: Vec<String>,       // container ids this runbook operates on
    pub steps: Vec<RunbookStep>,
}

#[derive(Debug, Clone)]
pub struct RunbookStep {
    pub description: String,
    pub command: Option<String>,
    pub check: Option<String>,
    pub rollback: Option<String>,
}
```

DSL:

```forge
runbook "restart-payments-api" {
  description "Zero-downtime restart of the payments API"
  url "https://runbooks.acme.internal/payments-api-restart"
  targets api

  step "Drain traffic via load balancer" {
    command "kubectl annotate svc/api drain=true"
    check "zero in-flight requests"
  }
  step "Restart pods" {
    command "kubectl rollout restart deployment/api"
    rollback "kubectl rollout undo deployment/api"
  }
  step "Restore traffic" {
    command "kubectl annotate svc/api drain-"
  }
}

views {
  flowView "restart-payments-api" "RestartRunbook" {
    title "Restart Payments API — Operational Runbook"
  }
}
```

**Verification.**

- Parser round-trip test for a 3-step runbook with mixed
  command/check/rollback fields.
- Layout test asserts each step ends up below its predecessor with
  the correct vertical gap.
- Render test asserts the SVG contains one box per step and one
  diamond node for each step that carries a `check` field.

**Open questions.**

- Cross-reference to external runbook URLs is on the runbook itself
  for now. Per-step URLs could be added later if teams ask.
- Should `targets` create a visible relationship in the container
  view ("container X has runbook Y")? First cut: no, keep the
  runbook view standalone. Add a container-side link in a follow-up.

---

## P2 items

### 9. SMIL animation output mode

**Priority:** P2 &nbsp; **Effort:** S &nbsp; **Depends on:** none

**Problem.** Animation today uses CSS keyframes + injected JS
([`forge/src/animate.rs`](../forge/src/animate.rs)). Some
environments strip JS: strict Content Security Policy headers,
GitHub's markdown renderer, plain PDF viewers, and many static-site
generators. SMIL (`<animate>`, `<set>`, `<animateTransform>`) runs
natively inside SVG without any JS at all. `DESIGN.md §3.7` lists
it as a planned rendering mode.

**Design.** Add `--animate <css|smil|none>` to `forge build`. `css`
stays the default. `smil` emits `<animate>` elements using discrete
value lists for step boundaries; the animation model (frames,
highlights, state changes) is preserved identically.

**Files to touch.**

- [`forge/src/animate.rs`](../forge/src/animate.rs) — new
  `animate_svg_smil` path alongside the existing `animate_svg`;
  share the frame-building logic, diverge only at emission.
- [`forge/src/main.rs`](../forge/src/main.rs) — `--animate` flag on
  the `Build` command.

**API sketch.**

```rust
// forge/src/animate.rs

pub enum AnimationMode {
    None,
    Css,
    Smil,
}

pub fn animate_svg_with_mode(
    svg: &str,
    view: &View,
    model: &Model,
    mode: AnimationMode,
) -> String {
    match mode {
        AnimationMode::None => svg.to_string(),
        AnimationMode::Css => animate_svg(svg, view, model),
        AnimationMode::Smil => animate_svg_smil(svg, view, model),
    }
}
```

CLI:

```bash
forge build --animate smil --view PaymentFlow --out out/
```

**Verification.**

- Render a known animated fixture in both `css` and `smil` modes;
  assert both contain the expected frame count.
- Assert the SMIL output contains no `<script>` tags and at least
  one `<animate>` element per frame.
- Regression: confirm the CSS output path is unchanged when
  `--animate` is omitted.

**Open questions.**

- SMIL is officially marked for deprecation in Chrome's roadmap but
  is still widely supported in every current browser. Document this
  caveat in the docs page; leave `css` as the default.

---

### 10. `!extends` / `!override` workspace inheritance

**Priority:** P2 &nbsp; **Effort:** S &nbsp; **Depends on:** none

**Problem.** `DESIGN.md §3.6` lists `!extends` and `!override` as
"Extending Workspaces" features. Today `!include` and `!fragment`
cover the most common cases, but there's no way to inherit a whole
workspace definition and selectively override parts of it — useful
for a `prod.forge` that reuses `base.forge` with environment-specific
overrides.

**Design.** Two new preprocessor directives in `preprocess.rs`.

- **`!extends "other.forge"`** — at the top of a file. Imports the
  other file's entire AST as the starting point for the current
  file. The extending file can then add new elements and override
  existing ones.
- **`!override <id> { ... }`** — inside a block. Replaces an
  element's body with the new one. Whole-element replacement to keep
  semantics obvious; no field-by-field merge.

**Files to touch.**

- [`forge/src/preprocess.rs`](../forge/src/preprocess.rs) — two new
  directive handlers.
- [`forge/examples/multi-file/`](../forge/examples/multi-file/) —
  add an `extends`/`override` example alongside the existing
  multi-file include example.

**API sketch.**

```forge
// base.forge
forge "Payments" {
  model {
    api = container "Payment API" {
      technology "Rust / Axum"
      description "Payment API (generic)"
    }
  }
}
```

```forge
// prod.forge
!extends "base.forge"

forge "Payments — Production" {
  !override api {
    technology "Rust / Axum (release build, PGO)"
    description "Payment API (production, hot path)"
    tags "production"
  }
}
```

**Verification.**

- Preprocessor unit test with a base and an extending file; assert
  the final AST has the overridden element's new description and
  keeps every other element from the base.
- Circular `!extends` detection reuses the existing circular-include
  logic from `!include`.

**Open questions.**

- Whole-element replacement is the simpler mental model. A
  field-level merge would be "smarter" but introduces a dozen
  corner cases around arrays, tags, and nested children. Ship
  whole-element first; revisit only if users hit a concrete case
  where it hurts.

---

### 11. `!include <url>` remote includes

**Priority:** P2 &nbsp; **Effort:** S (code) + design discussion
&nbsp; **Depends on:** none

**Problem.** Large orgs want to centralise shared definitions (team
names, common pipelines, shared trust boundaries) in one canonical
location and have every repo's `.forge` reference it. `DESIGN.md
§3.6` lists remote includes as a planned directive.

**Design.**

- Extend `!include` to accept `http://` / `https://` URLs.
- Fetched content is cached in `~/.cache/forge/includes/<sha256>.forge`.
- Gated behind `--allow-remote-includes` or the `FORGE_ALLOW_REMOTE_INCLUDES=1`
  env var to avoid accidental network calls in CI.
- Optional allowlist via `.forge-remotes` config file in the repo
  root, containing one URL prefix per line.
- Optional integrity pinning with an inline hash:
  `!include "url" sha256 "abc..."`. The preprocessor refuses the
  include if the hash doesn't match.

**Files to touch.**

- [`forge/src/preprocess.rs`](../forge/src/preprocess.rs) — URL
  detection in `!include` + HTTP fetch + cache.
- [`forge/Cargo.toml`](../forge/Cargo.toml) — add `ureq` (minimal
  blocking HTTP, pure Rust, no tokio dependency, TLS via `rustls`).
- [`forge/src/main.rs`](../forge/src/main.rs) —
  `--allow-remote-includes` flag on every command that preprocesses.

**API sketch.**

```forge
!include "https://raw.githubusercontent.com/acme/forge-shared/main/teams.forge"

!include "https://raw.githubusercontent.com/acme/forge-shared/main/pipelines.forge"
  sha256 "a1b2c3d4e5f6..."
```

**Verification.**

- Unit test with a mock HTTP server (e.g. `mockito`) serves a tiny
  fragment; assert the preprocessor inlines it correctly.
- Separate test that a remote include fails loudly (non-zero exit
  code with a clear error message) when the allow flag is off.
- Integrity test: hash-pinned include refuses a body with the wrong
  hash.

**Open questions.**

- Should remote includes resolve relative paths inside the fetched
  file? Plan: no — fetched files can only reference other remote
  URLs or absolute paths. Avoids confusing semantics.
- Cache expiry? Plan: never expire automatically; users clear via
  `rm -rf ~/.cache/forge/includes`. Simple and predictable.

---

### 12. Client-side search index for generated sites

**Priority:** P2 &nbsp; **Effort:** S &nbsp; **Depends on:** none

**Problem.** `forge generate` produces a static site with N pages
but no search. On any model with more than a dozen elements, users
struggle to find things. `DESIGN.md §10` medium-priority list
mentions "client-side search index for generated sites."

**Design.**

- Emit a `search-index.json` alongside the HTML. One entry per
  element and per view, with enough fields to drive substring
  matching on name + description.
- Add a small vanilla-JS search box to the site header (~50 lines
  of JS, no framework) that filters the index on keystroke and
  renders a dropdown of results.
- Results link to the existing element/view pages.

**Files to touch.**

- [`forge/src/generate.rs`](../forge/src/generate.rs) — emit
  `search-index.json` from the model.
- Templates — add a `<input type="search">` to the site header and
  a results container.
- New `search.js` embedded as a string constant in `generate.rs`
  (matching the existing approach for `style.css`).

**API sketch.** Output shape:

```json
[
  {
    "id": "api",
    "name": "Payment API",
    "kind": "container",
    "description": "REST + gRPC gateway",
    "href": "elements/api.html"
  },
  {
    "id": "Context",
    "name": "Payment Platform — System Context",
    "kind": "view",
    "description": "Actors and top-level systems",
    "href": "views/context.html"
  }
]
```

**Verification.**

- Integration test runs `forge generate` on `payments.forge` and
  asserts `_site/search-index.json` exists with one entry per
  element *plus* one per view. No JS-level tests needed; the search
  UI is plain substring filtering.

**Open questions.**

- Full-text over descriptions or name-only? Plan: substring over
  `name + description` joined with a space, case-insensitive.

---

## P3 items

### 13. GIF / WebM animation export

**Priority:** P3 &nbsp; **Effort:** L &nbsp; **Depends on:**
[item 1](#1-png--pdf-export-via-resvg) (raster rendering)

**Problem.** `DESIGN.md §3.7` promises GIF / WebM as animation
rendering modes. Value: embedding animated diagrams in
presentations, Slack threads, and issue trackers that strip JS and
sometimes SVG entirely. Today animation only runs in a live browser
preview.

**Design.** Rasterise each animation frame via `resvg` (item 1
provides `render_to_format` returning PNG bytes), then compose the
frames. Two paths:

- **GIF:** use the `image` crate, which has a pure-Rust GIF encoder.
- **WebM:** pure-Rust VP9 encoding is immature; the realistic path
  is an optional `--encoder ffmpeg` flag that shells out to
  `ffmpeg`. Document the dependency clearly.

**Files to touch.**

- [`forge/src/animate.rs`](../forge/src/animate.rs) — per-frame
  rasterisation loop that invokes `render::render_to_format` from
  item 1.
- [`forge/Cargo.toml`](../forge/Cargo.toml) — add `image` for GIF
  encoding.

**API sketch.**

```bash
forge build --format gif --source architecture.forge --view PaymentFlow --out out/
forge build --format webm --encoder ffmpeg --source architecture.forge --view PaymentFlow
```

**Verification.**

- Render a 3-frame animated fixture to GIF; assert file starts with
  `GIF89a` magic and contains the expected frame count in its
  header.
- WebM test is gated on `ffmpeg` being available in CI; skip
  gracefully when it isn't.

**Open questions.**

- Frame timing source? Pull from the existing animation frame
  duration; default to 1500 ms per frame.

---

### 14. Performance optimization for 10k-element models

**Priority:** P3 &nbsp; **Effort:** L &nbsp; **Depends on:** none

**Problem.** Forge is fast on the 30-element payments example. At
10,000 elements (a large enterprise landscape) the quadratic parts
of the layout and check passes will dominate. No user has hit this
yet; flagged as P3 until someone does.

**Design.** Profile first, optimise second.

- Add a `--bench` mode that generates a synthetic fixture of N
  elements (systems + containers + relationships) and prints
  layout + check timings per phase.
- Likely hotspots: N² relationship scans in
  [`check.rs`](../forge/src/check.rs) (e.g. `check_dependency_cycles`
  likely uses adjacency lookups that can be memoised),
  force-directed layout iterations once [item 3](#3-force-directed-layout--landscape-view-type)
  lands (Barnes-Hut is `O(N log N)` but the constant matters), and
  text measurement in [`text.rs`](../forge/src/text.rs) (which
  already caches but the cache may not cover every path).

**Files to touch.** TBD based on profiling. Likely suspects:
`check.rs`, `layout.rs`, `text.rs`.

**Verification.**

- Add `benches/` with `criterion`-based benchmarks that generate
  models of 100, 1k, and 10k elements and assert bounded time
  complexity. Fail the benchmark if p99 runtime regresses more than
  20 % across releases.

**Open questions.**

- Target: layout + check on 10k elements completes in under 5
  seconds on a mid-range laptop. Stretch goal: under 2 seconds.

---

### 15. Cross-compilation + release automation

**Priority:** P3 &nbsp; **Effort:** M &nbsp; **Depends on:** none

**Problem.** Forge is distributed via `cargo install forge-dsl`
only. Users without a Rust toolchain can't install it. `DESIGN.md
§10` lists "cross-compilation and release automation" as low
priority; still worth doing before any serious external adoption.

**Design.**

- Extend the existing [`.github/workflows/release.yml`](../.github/workflows/release.yml)
  to cross-compile for a platform matrix:
  `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`.
- Upload tarballs (Linux / macOS) and zips (Windows) as GitHub
  Release assets.
- Update the existing homebrew tap workflow to point at the new
  binaries rather than building from source.

**Files to touch.**

- [`.github/workflows/release.yml`](../.github/workflows/release.yml)
  — matrix expansion, per-platform build jobs, asset upload.
- [`forge/Cargo.toml`](../forge/Cargo.toml) — verify every dependency
  is cross-compile friendly. Flag anything pulling in `openssl-sys`
  (none today, but worth re-checking).

**Verification.**

- Run the release workflow against a dry-run tag and verify every
  platform produces a runnable binary.
- Smoke test per platform: `forge --version` exits 0 and prints a
  version string matching the tag.

**Open questions.**

- macOS code signing + notarisation is fiddly and requires an Apple
  Developer account. Document the steps but don't block v1 on it;
  users can self-sign with `codesign -s -` or use the Homebrew tap
  which handles signing.

---

### 16. Cargo workspace restructure

**Priority:** P3 &nbsp; **Effort:** M &nbsp; **Depends on:** none

**Problem.** Everything lives in one crate. `DESIGN.md §10` suggests
splitting into `forge-parser`, `forge-model`, `forge-analyze`,
`forge-render`, etc. Benefits: cleaner separation, shorter
incremental builds during development, and the ability for third
parties to depend on just the model without pulling in the MCP
server or LSP.

**Design.** Move code into a Cargo workspace structure. No
user-visible changes; purely organisational.

**Files to touch.** Essentially the entire source tree. This is why
it's P3 — big rearrangement with no direct user payoff.

**API sketch.**

```
forge/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── forge-core/             # model + parser + preprocess
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs
│   │       ├── parser.rs
│   │       └── preprocess.rs
│   ├── forge-analyze/          # analyze pipeline + correlate
│   ├── forge-render/           # layout + render + animate
│   ├── forge-lsp/              # LSP server
│   ├── forge-mcp/              # MCP server
│   └── forge-cli/              # thin orchestration binary
│       └── src/main.rs         # dispatches to the other crates
```

**Verification.**

- Every existing test continues to pass after the restructure.
- No functional change — the integration tests under
  `forge/src/analyze/fixture_tests.rs` all still pass with the same
  assertions.

**Open questions.**

- Keep the binary crate named `forge-dsl` for backwards
  compatibility on crates.io, or rename to `forge-cli`? Plan: keep
  `forge-dsl` — renaming would break every existing `cargo install`
  invocation and the reference CI workflow.

---

## Not in scope (deliberately)

These appeared during planning but were explicitly left off the
roadmap. Listed here so readers know they were considered and
rejected rather than forgotten.

- **Hugo / MkDocs plugins.** `DESIGN.md §6` describes them, but the
  current recommendation in [`forge/PUBLISHING.md`](../forge/PUBLISHING.md)
  is to call `forge build` from a CLI subprocess. No plugin code is
  needed — the existing integration pattern works.
- **CDK / Pulumi / Helm / Bicep scanners.** Orthogonal expansions
  of the `infra` scanner. Each would be its own PR; none are
  foundational. Add them as demand appears, one per PR.
- **GitLab CI / Jenkins / CircleCI / Buildkite scanners.** Parallel
  expansions of the `ci` scanner. Same story as above.
- **Swift / Elixir / Haskell language support.** These depend on
  `symgraph` adding upstream tree-sitter grammars, not on any
  change in forge itself.
- **APM / tracing integration.** Pulling service maps from Jaeger /
  Tempo / Datadog to upgrade guessed `imports` edges into observed
  `calls` edges would be a major feature, but it's speculative and
  depends on which APM vendors teams actually use. Revisit if users
  ask.

---

## Updating this document

- Move an item to "Recently shipped" as soon as it lands on `main`.
  Include the commit hash and a one-line summary.
- Re-prioritize freely. The P0/P1/P2/P3 tiers are a snapshot of
  current thinking, not a contract.
- When `DESIGN.md` and this document disagree, this document wins —
  but consider updating `DESIGN.md §10` in the same PR so the spec
  keeps its authoritative status.
- Keep item numbers stable. Don't renumber when an item ships;
  strike it through in the priority index and add a note pointing
  at the "Recently shipped" entry.
- Link checks: the docs site CI script validates every relative
  link. Don't break them.

---

## See also

- [`DESIGN.md`](../DESIGN.md) — the full design spec. Richer and
  more speculative than this document; source of truth for
  long-term direction.
- [`docs/README.md`](README.md) — documentation index.
- [`docs/reference/scanners.md`](reference/scanners.md) — what the
  analyze pipeline reads and produces today.
- [`docs/reference/correlations.md`](reference/correlations.md) —
  the cross-scanner correlate pass that powers the "Recently
  shipped" work above.

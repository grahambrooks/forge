"""
Combined site generator for the Forge evaluation framework.

Reads per-project JSON records from the baseline `results/` directory and
the Claude-driven `results-claude/` directory (either or both), regenerates
a `forge generate` sub-site for each `.forge` it finds, and writes a single
landing page at `<out>/index.html` that shows both runs side by side.

Project directory layout after a run:

    site/
    ├── index.html              # combined landing
    ├── styles.css
    ├── <name>/                 # baseline sub-site (when present)
    ├── <name>-claude/          # Claude sub-site (when present)
    └── compare.md              # copy of the markdown comparison (when both)

Output format is self-contained HTML — no runtime JS, no build step — so it
serves straight out of `python3 -m http.server` or GitHub Pages.

Python stdlib only.
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Iterable

from _lib import EVAL_DIR, log, run

# Default locations. The script uses these when no explicit paths are passed.
BASELINE_RESULTS = EVAL_DIR / "results"
CLAUDE_RESULTS = EVAL_DIR / "results-claude"
DEFAULT_SITE_OUT = BASELINE_RESULTS / "site"

# ── Forge DSL helpers (for augmenting view-less inferred models) ─
def _pick_id(model: dict, kind: str) -> str | None:
    for e in model.get("elements") or []:
        if e.get("kind") == kind:
            return e.get("id")
    return None


def _pick_strategy(model: dict) -> str | None:
    """Return the inferred branching-strategy name (e.g. "github-flow") from
    the first Branch element's `strategy` property, or None when no branches
    were emitted."""
    for e in model.get("elements") or []:
        if e.get("kind") == "Branch":
            strategy = (e.get("properties") or {}).get("strategy")
            if strategy:
                return strategy
    return None


def _load_strategy(results_dir: Path, name: str) -> str | None:
    path = results_dir / f"{name}.model.json"
    if not path.exists():
        return None
    try:
        return _pick_strategy(json.loads(path.read_text()))
    except json.JSONDecodeError:
        return None


def _views_block_for(model: dict) -> str:
    """Synthesize a `views { ... }` block targeting kinds present in `model`.

    Inferred models rarely carry an explicit views block; without one
    `forge generate` produces no diagrams. We add exactly enough so each
    sub-site has something to look at.
    """
    views: list[str] = []
    container_id = _pick_id(model, "Container")
    system_id = _pick_id(model, "System") or container_id
    pipeline_id = _pick_id(model, "Pipeline")
    deployment_id = _pick_id(model, "DeploymentNode")
    component_id = _pick_id(model, "Component")
    branching_id = _pick_id(model, "Branch")

    if system_id:
        views.append(
            f'    system-context-view {system_id} "SystemContext" {{\n'
            f"      include *\n      auto-layout lr\n"
            f'      title "System Context"\n    }}'
        )
    if container_id:
        views.append(
            f'    container-view {container_id} "Containers" {{\n'
            f"      include *\n      auto-layout tb\n"
            f'      title "Containers"\n    }}'
        )
    if component_id and container_id:
        views.append(
            f'    component-view {container_id} "Components" {{\n'
            f"      include *\n      auto-layout tb\n"
            f'      title "Components"\n    }}'
        )
    if pipeline_id:
        views.append(
            f'    pipeline-view {pipeline_id} "Pipeline" {{\n'
            f"      include *\n      auto-layout lr\n"
            f'      title "CI/CD Pipeline"\n    }}'
        )
    if deployment_id:
        views.append(
            f'    deployment-view {deployment_id} "Deployment" {{\n'
            f"      include *\n      auto-layout tb\n"
            f'      title "Deployment"\n    }}'
        )
    if branching_id:
        views.append(
            f'    branching-view {branching_id} "Branching" {{\n'
            f"      include *\n      auto-layout tb\n"
            f'      title "Branching Strategy"\n    }}'
        )
    if model.get("tech-stack") or []:
        views.append(
            '    tech-stack-view "TechStack" {\n      include *\n'
            '      title "Technology Stack"\n    }'
        )
    if model.get("teams") or []:
        views.append(
            '    team-view "Teams" {\n      include *\n'
            '      title "Team Ownership"\n    }'
        )
    if not views:
        return ""
    return "\n  views {\n" + "\n\n".join(views) + "\n  }\n"


def _augment_with_views(forge_path: Path, model: dict) -> Path:
    """If the .forge has no views block, return a sibling `.viewed.forge`
    with a synthesized one. No-op when views already exist."""
    original = forge_path.read_text()
    if "views {" in original:
        return forge_path
    block = _views_block_for(model)
    if not block:
        return forge_path
    last_close = original.rfind("}")
    if last_close < 0:
        return forge_path
    augmented = original[:last_close] + block + original[last_close:]
    out = forge_path.with_suffix(".viewed.forge")
    out.write_text(augmented)
    return out


# ── `forge generate` driver ─────────────────────────────────────
def _generate_subsite(
    *,
    sub_out: Path,
    forge_path: Path,
    forge_bin: Path,
    title: str,
    log_path: Path,
    timeout: float = 120,
) -> tuple[Path | None, str | None]:
    """Run `forge generate --source <forge_path> --out <sub_out>`."""
    if sub_out.exists():
        shutil.rmtree(sub_out)
    sub_out.mkdir(parents=True, exist_ok=True)

    code, stdout, stderr = run(
        [
            str(forge_bin),
            "generate",
            "--source",
            str(forge_path),
            "--out",
            str(sub_out),
            "--title",
            title,
        ],
        timeout=timeout,
    )
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(f"$ forge generate {forge_path} --out {sub_out}\n\n{stdout}\n{stderr}")
    if code != 0:
        last = stderr.strip().splitlines()[-1] if stderr.strip() else "no stderr"
        return None, f"generate exit {code}: {last}"
    return sub_out, None


# ── Preview SVG ─────────────────────────────────────────────────
def _read_preview_svg(site_dir: Path) -> str | None:
    diagrams = site_dir / "assets" / "diagrams"
    if not diagrams.exists():
        return None
    for preferred in (
        "Containers.svg",
        "SystemContext.svg",
        "Deployment.svg",
        "Pipeline.svg",
    ):
        p = diagrams / preferred
        if p.exists():
            return p.read_text()
    for svg in sorted(diagrams.glob("*.svg")):
        return svg.read_text()
    return None


def _strip_xml_prolog(svg: str) -> str:
    svg = svg.lstrip()
    if svg.startswith("<?xml"):
        end = svg.find("?>")
        if end >= 0:
            svg = svg[end + 2 :].lstrip()
    return svg


def _html_escape(s: str) -> str:
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


# ── Result loading ──────────────────────────────────────────────
def _load_results(results_dir: Path) -> dict[str, dict]:
    """Return `{project_name: envelope-dict}` from a results directory.

    Reads the generic envelope (whatever shape the driver wrote) rather than
    reconstructing into typed dataclasses — this module doesn't need to
    couple to either driver's in-memory representation.
    """
    out: dict[str, dict] = {}
    if not results_dir.exists():
        return out
    for p in sorted(results_dir.glob("*.json")):
        if "transcript" in p.name or "model" in p.name or p.name == "report.json":
            continue
        try:
            d = json.loads(p.read_text())
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict) or "status" not in d:
            continue
        out[d["name"]] = d
    return out


def _elem_total(entry: dict | None) -> int:
    if not entry:
        return 0
    return sum((entry.get("metrics") or {}).get("element_counts", {}).values())


def _metric(entry: dict | None, key: str, default=0):
    if not entry:
        return default
    return (entry.get("metrics") or {}).get(key, default)


def _wall(entry: dict | None, key: str) -> float:
    if not entry:
        return 0.0
    return float((entry.get("timings") or {}).get(key, 0.0))


def _status_label(s: str | None) -> str:
    return (s or "—").upper() if s != "error" else "ERR"


# ── Styles (inline-friendly) ────────────────────────────────────
#
# Two themes driven by `prefers-color-scheme`. Every page uses CSS
# custom properties so the same markup adapts to whatever the OS is set
# to — a previous version of the site mixed light defaults with the
# forge SVGs' dark-mode CSS, so dark-mode visitors saw light text on a
# light background. The forge SVGs themselves already dark-adapt (see
# `forge/src/render/css.rs`), so as long as our chrome follows suit
# everything stays legible.
INDEX_CSS = """\
:root {
    color-scheme: light dark;
    --bg: #fafbfc;
    --bg-card: #ffffff;
    --bg-muted: #f7f8fa;
    --bg-th: #f3f5f7;
    --bg-group-base: #eef2f6;
    --bg-group-claude: #ede9fe;
    --bg-tag-default: #eef0f3;
    --bg-preview: #ffffff;
    --fg: #1f2430;
    --fg-muted: #4b5563;
    --fg-faint: #6b7280;
    --fg-quiet: #9ca3af;
    --border: #e4e7eb;
    --border-claude: #ddd6fe;
    --pass-bg: #d4edda;     --pass-fg: #155724;
    --fail-bg: #fff3cd;     --fail-fg: #856404;
    --error-bg: #f8d7da;    --error-fg: #721c24;
    --claude-bg: #e0e7ff;   --claude-fg: #3730a3;
    --claude-card: #f5f3ff;
    --claude-h4: #5b21b6;
}
@media (prefers-color-scheme: dark) {
    :root {
        --bg: #0d1117;
        --bg-card: #161b22;
        --bg-muted: #1c222b;
        --bg-th: #1c222b;
        --bg-group-base: #1c2330;
        --bg-group-claude: #2a1f48;
        --bg-tag-default: #21262d;
        /* The SVG previews' own background is var(--svg-bg), which the
           forge CSS already flips to a dark value in dark mode. Match
           that here so the preview frame doesn't show as a light tile
           around dark-themed diagrams. */
        --bg-preview: #0d1117;
        --fg: #e6e8ec;
        --fg-muted: #c9d1da;
        --fg-faint: #8b949e;
        --fg-quiet: #6e7681;
        --border: #30363d;
        --border-claude: #4c1d95;
        --pass-bg: #14361f;     --pass-fg: #7ee2a8;
        --fail-bg: #3a2e10;     --fail-fg: #ffd97a;
        --error-bg: #3b1418;    --error-fg: #f5a3aa;
        --claude-bg: #2a2752;   --claude-fg: #c4b5fd;
        --claude-card: #1f1840;
        --claude-h4: #c4b5fd;
    }
}
body { font: 14px/1.5 -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
       margin: 0; padding: 2rem; max-width: 1400px;
       color: var(--fg); background: var(--bg); }
a { color: inherit; }
h1 { margin-top: 0; }
h2 { margin-top: 2.5rem; border-bottom: 1px solid var(--border); padding-bottom: 0.3rem; }
.lede { color: var(--fg-muted); max-width: 900px; }
.summary { display: flex; gap: 1rem; flex-wrap: wrap; margin: 1rem 0 2rem; }
.summary .tag { padding: 0.3rem 0.8rem; border-radius: 4px; font-weight: 600;
                background: var(--bg-tag-default); color: var(--fg); }
.tag.pass { background: var(--pass-bg); color: var(--pass-fg); }
.tag.fail { background: var(--fail-bg); color: var(--fail-fg); }
.tag.error { background: var(--error-bg); color: var(--error-fg); }
.tag.claude { background: var(--claude-bg); color: var(--claude-fg); }
table.summary-table { border-collapse: collapse; width: 100%; margin-bottom: 2rem; }
table.summary-table th, table.summary-table td {
    border-bottom: 1px solid var(--border); padding: 0.5rem 0.6rem; text-align: left;
    font-size: 13px;
}
table.summary-table th { background: var(--bg-th); font-weight: 600; }
table.summary-table th.group-base { background: var(--bg-group-base); border-left: 3px solid var(--fg-faint); }
table.summary-table th.group-claude { background: var(--bg-group-claude); border-left: 3px solid #6d28d9; }
table.summary-table td.group-base { border-left: 3px solid var(--border); }
table.summary-table td.group-claude { border-left: 3px solid var(--border-claude); }
.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(560px, 1fr));
         gap: 1.5rem; }
.card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px;
        padding: 1rem 1.25rem; }
.card > h3 { margin: 0 0 0.25rem; font-size: 1.05rem; }
.card .meta { color: var(--fg-faint); font-size: 0.85rem; margin-bottom: 0.75rem; }
.run-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.run-col { background: var(--bg-muted); border-radius: 6px; padding: 0.6rem 0.7rem; }
.run-col.claude { background: var(--claude-card); }
.run-col h4 { margin: 0 0 0.3rem; font-size: 0.85rem; font-weight: 600;
              text-transform: uppercase; letter-spacing: 0.05em;
              color: var(--fg-muted); }
.run-col.claude h4 { color: var(--claude-h4); }
.run-col .preview { background: var(--bg-preview); border-radius: 4px; padding: 0.4rem;
                    max-height: 200px; overflow: hidden; display: flex;
                    justify-content: center; align-items: center;
                    margin-bottom: 0.4rem; }
.run-col .preview svg { max-width: 100%; max-height: 180px; height: auto; }
.run-col .no-preview { color: var(--fg-quiet); font-style: italic; padding: 1.5rem 0;
                       font-size: 0.85rem; text-align: center; }
.run-col .stat { font-size: 0.8rem; color: var(--fg-muted); }
.run-col .links { margin-top: 0.3rem; font-size: 0.8rem; }
.run-col .links a { margin-right: 0.6rem; }
.badge { display: inline-block; padding: 0.1rem 0.5rem; border-radius: 3px;
         font-size: 0.7rem; font-weight: 600; margin-left: 0.4rem;
         text-transform: uppercase; letter-spacing: 0.03em; }
.badge.pass { background: var(--pass-bg); color: var(--pass-fg); }
.badge.fail { background: var(--fail-bg); color: var(--fail-fg); }
.badge.error { background: var(--error-bg); color: var(--error-fg); }
.badge.missing { background: var(--bg-tag-default); color: var(--fg-faint); }
.delta-pos { color: var(--pass-fg); }
.delta-neg { color: var(--error-fg); }
.delta-zero { color: var(--fg-quiet); }
code { background: var(--bg-muted); padding: 0.1rem 0.35rem; border-radius: 3px; }
footer { margin-top: 3rem; color: var(--fg-quiet); font-size: 0.85rem; }
"""


# ── Per-project site generation ─────────────────────────────────
def _build_subsite_for(
    *,
    name: str,
    kind: str,  # "baseline" | "claude"
    entry: dict,
    src_results_dir: Path,
    site_dir: Path,
    forge_bin: Path,
) -> tuple[Path | None, str | None, str | None]:
    """Generate a sub-site for one project+run. Returns (out_dir, preview_svg, error)."""
    forge_path = src_results_dir / f"{name}.forge"
    model_path = src_results_dir / f"{name}.model.json"
    if entry.get("status") == "error":
        return None, None, entry.get("error") or "run errored"
    if not forge_path.exists():
        return None, None, "no .forge file produced"
    if not model_path.exists():
        return None, None, "no model.json (export failed?)"

    try:
        model = json.loads(model_path.read_text())
    except json.JSONDecodeError as e:
        return None, None, f"model.json invalid: {e}"

    augmented = _augment_with_views(forge_path, model)
    sub_name = name if kind == "baseline" else f"{name}-claude"
    title = f"forge — {name}{' (Claude)' if kind == 'claude' else ''}"
    out, err = _generate_subsite(
        sub_out=site_dir / sub_name,
        forge_path=augmented,
        forge_bin=forge_bin,
        title=title,
        log_path=src_results_dir / f"{name}.generate.log",
    )
    preview = _read_preview_svg(out) if out else None
    return out, preview, err


# ── Landing page ────────────────────────────────────────────────
def _delta_span(base: int, claude: int) -> str:
    if claude == 0 and base == 0:
        return "<span class='delta-zero'>—</span>"
    d = claude - base
    if d == 0:
        return "<span class='delta-zero'>0</span>"
    cls = "delta-pos" if d > 0 else "delta-neg"
    return f"<span class='{cls}'>{d:+d}</span>"


def _status_badge(status: str | None) -> str:
    if not status:
        return "<span class='badge missing'>—</span>"
    return f"<span class='badge {status}'>{status}</span>"


def _render_summary_tags(baseline: dict[str, dict], claude: dict[str, dict]) -> str:
    """Top-of-page pill summary."""
    def _counts(d: dict[str, dict]) -> tuple[int, int, int, int]:
        return (
            len(d),
            sum(1 for r in d.values() if r.get("status") == "pass"),
            sum(1 for r in d.values() if r.get("status") == "fail"),
            sum(1 for r in d.values() if r.get("status") == "error"),
        )

    base_total, base_pass, base_fail, base_err = _counts(baseline)
    cla_total, cla_pass, cla_fail, cla_err = _counts(claude)
    total_cost = sum(
        float((r.get("usage") or {}).get("cost_usd", 0.0)) for r in claude.values()
    )

    parts: list[str] = []
    if baseline:
        parts.append(f"<span class='tag'>baseline: {base_total}</span>")
        parts.append(f"<span class='tag pass'>{base_pass} pass</span>")
        if base_fail:
            parts.append(f"<span class='tag fail'>{base_fail} fail</span>")
        if base_err:
            parts.append(f"<span class='tag error'>{base_err} error</span>")
    if claude:
        parts.append(f"<span class='tag claude'>claude: {cla_total}</span>")
        parts.append(f"<span class='tag pass'>{cla_pass} pass</span>")
        if cla_fail:
            parts.append(f"<span class='tag fail'>{cla_fail} fail</span>")
        if cla_err:
            parts.append(f"<span class='tag error'>{cla_err} error</span>")
        parts.append(f"<span class='tag'>${total_cost:.2f} total</span>")
    return f"<div class='summary'>{''.join(parts)}</div>"


def _render_summary_table(
    projects: Iterable[str],
    baseline: dict[str, dict],
    claude: dict[str, dict],
    baseline_sites: dict[str, Path | None],
    claude_sites: dict[str, Path | None],
    strategies: dict[str, str | None],
) -> str:
    """Combined table with grouped Baseline / Claude columns."""
    has_claude = bool(claude)

    headers = [
        "<th rowspan='2'>Project</th>",
        "<th rowspan='2'>Tier</th>",
        "<th rowspan='2'>Strategy</th>",
        "<th class='group-base' colspan='4'>Baseline (forge analyze)</th>",
    ]
    if has_claude:
        headers.append("<th class='group-claude' colspan='5'>Claude (forge-architect)</th>")
    headers.append("<th rowspan='2'>Δ elements</th>" if has_claude else "")

    sub_headers = [
        "<th class='group-base'>Status</th>",
        "<th class='group-base'>Elements</th>",
        "<th class='group-base'>Views</th>",
        "<th class='group-base'>Time</th>",
    ]
    if has_claude:
        sub_headers += [
            "<th class='group-claude'>Status</th>",
            "<th class='group-claude'>Elements</th>",
            "<th class='group-claude'>Views</th>",
            "<th class='group-claude'>Time</th>",
            "<th class='group-claude'>Cost</th>",
        ]

    rows: list[str] = []
    for name in projects:
        b = baseline.get(name)
        c = claude.get(name) if has_claude else None
        tier = int((b or c or {}).get("tier", 0))
        be = _elem_total(b)
        ce = _elem_total(c)

        site_b = baseline_sites.get(name)
        site_c = claude_sites.get(name)
        name_cell = _html_escape(name)
        if site_b:
            name_cell = f"<a href='./{name}/index.html'>{name_cell}</a>"
        strategy = strategies.get(name)
        strategy_cell = (
            f"<code>{_html_escape(strategy)}</code>"
            if strategy
            else "<span class='delta-zero'>—</span>"
        )
        tr = [
            f"<td>{name_cell}</td>",
            f"<td>{tier}</td>",
            f"<td>{strategy_cell}</td>",
        ]

        # Baseline group
        tr.append(f"<td class='group-base'>{_status_badge(b.get('status') if b else None)}</td>")
        tr.append(f"<td class='group-base'>{be if b else '—'}</td>")
        tr.append(f"<td class='group-base'>{_metric(b, 'views', 0) if b else '—'}</td>")
        tr.append(
            f"<td class='group-base'>{_wall(b, 'analyze_seconds'):.1f}s</td>"
            if b
            else "<td class='group-base'>—</td>"
        )

        # Claude group
        if has_claude:
            if c:
                cost = float((c.get("usage") or {}).get("cost_usd", 0.0))
                tr.append(
                    f"<td class='group-claude'>{_status_badge(c.get('status'))}"
                    + (
                        f" <a href='./{name}-claude/index.html'>↗</a>"
                        if site_c
                        else ""
                    )
                    + "</td>"
                )
                tr.append(f"<td class='group-claude'>{ce}</td>")
                tr.append(f"<td class='group-claude'>{_metric(c, 'views', 0)}</td>")
                tr.append(f"<td class='group-claude'>{_wall(c, 'claude_seconds'):.1f}s</td>")
                tr.append(f"<td class='group-claude'>${cost:.3f}</td>")
            else:
                tr.extend(["<td class='group-claude'>—</td>"] * 5)

            tr.append(f"<td>{_delta_span(be, ce)}</td>")

        rows.append("<tr>" + "".join(tr) + "</tr>")

    return (
        "<table class='summary-table'>"
        + "<thead>"
        + "<tr>" + "".join(h for h in headers if h) + "</tr>"
        + "<tr>" + "".join(sub_headers) + "</tr>"
        + "</thead><tbody>"
        + "".join(rows)
        + "</tbody></table>"
    )


def _render_card(
    name: str,
    *,
    baseline: dict | None,
    claude: dict | None,
    baseline_preview: str | None,
    claude_preview: str | None,
    baseline_site: Path | None,
    claude_site: Path | None,
    has_claude_run: bool,
) -> str:
    tier = int((baseline or claude or {}).get("tier", 0))
    be = _elem_total(baseline)
    ce = _elem_total(claude)

    header = (
        f"<h3>{_html_escape(name)}"
        f"<span class='meta' style='margin-left:0.5rem;'>tier {tier}</span></h3>"
        f"<div class='meta'>"
        f"baseline: {be} elements · "
        f"claude: {ce} elements (Δ {_delta_span(be, ce)})"
        "</div>"
        if has_claude_run
        else (
            f"<h3>{_html_escape(name)}"
            f"<span class='meta' style='margin-left:0.5rem;'>tier {tier}</span></h3>"
            f"<div class='meta'>{be} elements · "
            f"{_metric(baseline, 'relationships', 0)} relationships · "
            f"{_wall(baseline, 'analyze_seconds'):.1f}s</div>"
        )
    )

    def _col(kind: str, entry: dict | None, preview: str | None, site: Path | None) -> str:
        label = "Baseline" if kind == "baseline" else "Claude"
        cls = "run-col claude" if kind == "claude" else "run-col"
        if entry is None:
            return (
                f"<div class='{cls}'>"
                f"<h4>{label}</h4>"
                f"<div class='no-preview'>not run</div>"
                "</div>"
            )
        preview_html = (
            _strip_xml_prolog(preview)
            if preview
            else f"<div class='no-preview'>{_html_escape(entry.get('error') or 'no diagrams')}</div>"
        )
        status = entry.get("status", "")
        if kind == "baseline":
            stat_line = (
                f"{_elem_total(entry)} elements · "
                f"{_metric(entry, 'relationships', 0)} rel · "
                f"{_wall(entry, 'analyze_seconds'):.1f}s"
            )
        else:
            cost = float((entry.get("usage") or {}).get("cost_usd", 0.0))
            turns = int((entry.get("usage") or {}).get("turns", 0))
            stat_line = (
                f"{_elem_total(entry)} elements · "
                f"{_wall(entry, 'claude_seconds'):.1f}s · "
                f"${cost:.3f} · {turns} turns"
            )
        links: list[str] = []
        sub_name = name if kind == "baseline" else f"{name}-claude"
        if site:
            links.append(f"<a href='./{sub_name}/index.html'>full site</a>")
            links.append(f"<a href='./{sub_name}/forge.json'>model JSON</a>")
        return (
            f"<div class='{cls}'>"
            f"<h4>{label} {_status_badge(status)}</h4>"
            f"<div class='preview'>{preview_html}</div>"
            f"<div class='stat'>{stat_line}</div>"
            f"<div class='links'>{' '.join(links)}</div>"
            "</div>"
        )

    cols = [_col("baseline", baseline, baseline_preview, baseline_site)]
    if has_claude_run:
        cols.append(_col("claude", claude, claude_preview, claude_site))

    return (
        "<div class='card'>"
        + header
        + "<div class='run-row'>"
        + "".join(cols)
        + "</div>"
        + "</div>"
    )


# ── Public entry ────────────────────────────────────────────────
def build_site(
    *,
    forge_bin: Path,
    out_dir: Path = DEFAULT_SITE_OUT,
    baseline_dir: Path = BASELINE_RESULTS,
    claude_dir: Path = CLAUDE_RESULTS,
    skip_regenerate: bool = False,
) -> Path:
    """Build the combined static site.

    - Reads baseline and Claude results from their respective directories.
    - Regenerates per-project sub-sites (baseline → `site/<name>/`, claude →
      `site/<name>-claude/`) via `forge generate`. Skipped when
      `skip_regenerate=True` (useful for quick landing-page iteration).
    - Writes `<out>/index.html` with a combined landing page.
    - Copies `results-claude/compare.md` into the site if present.

    Returns the output directory.
    """
    baseline = _load_results(baseline_dir)
    claude = _load_results(claude_dir)

    out_dir.mkdir(parents=True, exist_ok=True)

    baseline_sites: dict[str, Path | None] = {}
    baseline_previews: dict[str, str | None] = {}
    claude_sites: dict[str, Path | None] = {}
    claude_previews: dict[str, str | None] = {}
    # Strategy is a per-project property of the analyzer output, not of the
    # run — both baseline and Claude analyse the same repo. Prefer the
    # baseline strategy, falling back to Claude's when baseline didn't run.
    strategies: dict[str, str | None] = {}

    forge_css: str | None = None  # captured from the first generated sub-site

    all_names = sorted(set(baseline) | set(claude), key=lambda n: (
        int((baseline.get(n) or claude.get(n) or {}).get("tier", 0)),
        n,
    ))

    for name in all_names:
        b = baseline.get(name)
        c = claude.get(name)
        strategies[name] = (
            _load_strategy(baseline_dir, name)
            or _load_strategy(claude_dir, name)
        )
        if b is not None:
            if skip_regenerate:
                existing = out_dir / name
                site = existing if existing.exists() else None
                baseline_sites[name] = site
                baseline_previews[name] = _read_preview_svg(site) if site else None
            else:
                log(f"  site: {name} (baseline)")
                site, preview, err = _build_subsite_for(
                    name=name,
                    kind="baseline",
                    entry=b,
                    src_results_dir=baseline_dir,
                    site_dir=out_dir,
                    forge_bin=forge_bin,
                )
                if err:
                    log(f"    ✗ {err}")
                baseline_sites[name] = site
                baseline_previews[name] = preview
                if site and forge_css is None:
                    css_path = site / "assets" / "forge.css"
                    if css_path.exists():
                        forge_css = css_path.read_text()

        if c is not None:
            if skip_regenerate:
                existing = out_dir / f"{name}-claude"
                site = existing if existing.exists() else None
                claude_sites[name] = site
                claude_previews[name] = _read_preview_svg(site) if site else None
            else:
                log(f"  site: {name} (claude)")
                site, preview, err = _build_subsite_for(
                    name=name,
                    kind="claude",
                    entry=c,
                    src_results_dir=claude_dir,
                    site_dir=out_dir,
                    forge_bin=forge_bin,
                )
                if err:
                    log(f"    ✗ {err}")
                claude_sites[name] = site
                claude_previews[name] = preview
                if site and forge_css is None:
                    css_path = site / "assets" / "forge.css"
                    if css_path.exists():
                        forge_css = css_path.read_text()

    # Copy compare.md over if the companion driver produced one; keep it
    # linkable from the landing page.
    compare_src = claude_dir / "compare.md"
    if compare_src.exists():
        shutil.copyfile(compare_src, out_dir / "compare.md")

    _write_index(
        out_dir=out_dir,
        baseline=baseline,
        claude=claude,
        baseline_sites=baseline_sites,
        claude_sites=claude_sites,
        baseline_previews=baseline_previews,
        claude_previews=claude_previews,
        strategies=strategies,
        forge_css=forge_css,
    )
    return out_dir


def _write_index(
    *,
    out_dir: Path,
    baseline: dict[str, dict],
    claude: dict[str, dict],
    baseline_sites: dict[str, Path | None],
    claude_sites: dict[str, Path | None],
    baseline_previews: dict[str, str | None],
    claude_previews: dict[str, str | None],
    strategies: dict[str, str | None],
    forge_css: str | None,
) -> None:
    (out_dir / "styles.css").write_text(INDEX_CSS)

    all_names = sorted(
        set(baseline) | set(claude),
        key=lambda n: (int((baseline.get(n) or claude.get(n) or {}).get("tier", 0)), n),
    )

    has_claude_run = bool(claude)

    parts: list[str] = []
    parts.append("<!DOCTYPE html><html lang='en'><head><meta charset='utf-8'>")
    parts.append("<meta name='viewport' content='width=device-width, initial-scale=1'>")
    parts.append("<title>Forge — evaluation results</title>")
    parts.append("<link rel='stylesheet' href='./styles.css'>")
    if forge_css:
        parts.append(f"<style>\n{forge_css}\n</style>")
    parts.append("</head><body>")

    if has_claude_run:
        parts.append("<h1>Forge — baseline vs Claude-driven evaluation</h1>")
        parts.append(
            "<p class='lede'>"
            "Two runs over the same corpus. "
            "<strong>Baseline</strong> invokes <code>forge analyze</code> directly. "
            "<strong>Claude</strong> invokes Claude Code headless with the "
            "<code>forge-architect</code> plugin, which calls the analyzer via "
            "the Forge MCP server and then reviews the result. Click a project "
            "to drill into either sub-site."
            "</p>"
        )
    else:
        parts.append("<h1>Forge — <code>forge analyze</code> evaluation</h1>")
        parts.append(
            "<p class='lede'>"
            "Results from running <code>forge analyze</code> against the pinned "
            "corpus. Click a project name for its drill-down site."
            "</p>"
        )

    parts.append(_render_summary_tags(baseline, claude))

    if (out_dir / "compare.md").exists():
        parts.append(
            "<p><a href='./compare.md'>Full comparison report (markdown)</a></p>"
        )

    parts.append("<h2>Summary</h2>")
    parts.append(
        _render_summary_table(
            all_names,
            baseline,
            claude,
            baseline_sites,
            claude_sites,
            strategies,
        )
    )

    parts.append("<h2>Per-project preview</h2>")
    parts.append("<div class='cards'>")
    for name in all_names:
        parts.append(
            _render_card(
                name,
                baseline=baseline.get(name),
                claude=claude.get(name),
                baseline_preview=baseline_previews.get(name),
                claude_preview=claude_previews.get(name),
                baseline_site=baseline_sites.get(name),
                claude_site=claude_sites.get(name),
                has_claude_run=has_claude_run,
            )
        )
    parts.append("</div>")

    footer = (
        "Generated by eval/sitegen.py. Source corpus: corpus.json · "
        "Run with <code>python3 eval/run.py</code> (baseline) and "
        "<code>python3 eval/run_claude.py</code> (Claude)."
    )
    parts.append(f"<footer>{footer}</footer>")
    parts.append("</body></html>")

    index_path = out_dir / "index.html"
    index_path.write_text("\n".join(parts))
    log(f"Wrote {index_path}")


# ── CLI ──────────────────────────────────────────────────────────
def main() -> int:
    import argparse

    from _lib import find_forge_binary

    ap = argparse.ArgumentParser(
        description="Build the combined baseline+Claude evaluation site"
    )
    ap.add_argument("--forge", help="path to forge binary")
    ap.add_argument("--out", default=str(DEFAULT_SITE_OUT), help="output directory")
    ap.add_argument("--baseline-dir", default=str(BASELINE_RESULTS))
    ap.add_argument("--claude-dir", default=str(CLAUDE_RESULTS))
    ap.add_argument(
        "--skip-regenerate",
        action="store_true",
        help="don't re-run forge generate; only rewrite index.html",
    )
    args = ap.parse_args()

    forge_bin = find_forge_binary(args.forge)
    out = build_site(
        forge_bin=forge_bin,
        out_dir=Path(args.out),
        baseline_dir=Path(args.baseline_dir),
        claude_dir=Path(args.claude_dir),
        skip_regenerate=args.skip_regenerate,
    )
    log(f"Site at {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

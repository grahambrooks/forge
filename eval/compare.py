#!/usr/bin/env python3
"""
Compare baseline (`forge analyze`) vs Claude-driven (forge-architect plugin)
results on the same corpus.

Reads `results/<name>.json` (from run.py) and `results-claude/<name>.json`
(from run_claude.py), pairs them, and emits a side-by-side markdown report
at `results-claude/compare.md`.

The comparison is intentionally narrow: both runs produce a .forge file
and we parse both through `forge export` with identical metrics. Any
difference therefore reflects *what ended up in the file*, not how it was
measured. In practice the Claude-driven run calls the same `forge analyze`
underneath the MCP, so the headline numbers usually match — the interesting
deltas are when Claude resolves a check violation or the analyzer succeeds
where Claude's budget ran out (or vice versa).

Usage:
    ./compare.py                # write results-claude/compare.md
    ./compare.py --stdout       # print to stdout instead

Python stdlib only.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

from _lib import EVAL_DIR, log

BASELINE_DIR = EVAL_DIR / "results"
CLAUDE_DIR = EVAL_DIR / "results-claude"
COMPARE_PATH = CLAUDE_DIR / "compare.md"


@dataclass
class Row:
    name: str
    tier: int
    baseline: dict | None
    claude: dict | None


# ── Result loading ───────────────────────────────────────────────
def _load_dir(results_dir: Path) -> dict[str, dict]:
    """Return a `{name: dict}` map of per-project envelopes.

    We read them generically as dicts rather than the typed dataclasses so
    this script doesn't couple to either driver's in-memory representation
    — either schema can evolve independently.
    """
    out: dict[str, dict] = {}
    if not results_dir.exists():
        return out
    for p in sorted(results_dir.glob("*.json")):
        if "transcript" in p.name or "model" in p.name or p.name in ("report.json",):
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


def _delta(a: int, b: int) -> str:
    """Render `b - a` with a sign prefix and a hint when they agree."""
    if a == b:
        return "0"
    d = b - a
    return f"{d:+d}"


def _status_icon(s: str) -> str:
    return {"pass": "✓", "fail": "○", "error": "✗"}.get(s or "", "?")


# ── Report ───────────────────────────────────────────────────────
def build_report(rows: list[Row]) -> str:
    lines: list[str] = []
    lines.append("# Forge eval — baseline vs Claude-driven")
    lines.append("")

    have_both = [r for r in rows if r.baseline and r.claude]
    only_base = [r for r in rows if r.baseline and not r.claude]
    only_claude = [r for r in rows if r.claude and not r.baseline]

    lines.append(
        f"- **{len(have_both)}** projects in both runs"
    )
    if only_base:
        lines.append(
            f"- **{len(only_base)}** only in baseline: "
            f"{', '.join(r.name for r in only_base)}"
        )
    if only_claude:
        lines.append(
            f"- **{len(only_claude)}** only in Claude run: "
            f"{', '.join(r.name for r in only_claude)}"
        )
    total_cost = sum(
        (r.claude or {}).get("usage", {}).get("cost_usd", 0.0) for r in rows
    )
    lines.append(f"- **${total_cost:.2f}** total Claude spend")
    lines.append("")

    # Headline table ─ compare the two runs on the intersection.
    lines.append("## Side by side")
    lines.append("")
    lines.append(
        "| Project | Status (base / claude) | Elements (base / claude / Δ) | "
        "Relations (Δ) | Views (Δ) | Wall (base / claude) | $ claude |"
    )
    lines.append("| --- | --- | --- | --- | --- | --- | --- |")
    for r in sorted(have_both, key=lambda x: (x.tier, x.name)):
        base = r.baseline or {}
        cl = r.claude or {}
        be = _elem_total(base)
        ce = _elem_total(cl)
        br = _metric(base, "relationships", 0)
        cr = _metric(cl, "relationships", 0)
        bv = _metric(base, "views", 0)
        cv = _metric(cl, "views", 0)
        base_wall = _wall(base, "analyze_seconds")
        claude_wall = _wall(cl, "claude_seconds")
        cost = (cl.get("usage") or {}).get("cost_usd", 0.0)
        lines.append(
            f"| {r.name} | "
            f"{_status_icon(base.get('status', ''))} / {_status_icon(cl.get('status', ''))} | "
            f"{be} / {ce} ({_delta(be, ce)}) | "
            f"{br} / {cr} ({_delta(br, cr)}) | "
            f"{bv} / {cv} ({_delta(bv, cv)}) | "
            f"{base_wall:.1f}s / {claude_wall:.1f}s | "
            f"${cost:.3f} |"
        )
    lines.append("")

    # Provenance alignment — where Claude omitted a scanner or produced one
    # the baseline didn't.
    lines.append("## Provenance deltas")
    lines.append("")
    lines.append("| Project | Baseline scanners | Claude scanners | Missing (Claude) | Extra (Claude) |")
    lines.append("| --- | --- | --- | --- | --- |")
    for r in sorted(have_both, key=lambda x: (x.tier, x.name)):
        b_prov = set((r.baseline.get("metrics") or {}).get("provenance") or {})
        c_prov = set((r.claude.get("metrics") or {}).get("provenance") or {})
        missing = b_prov - c_prov
        extra = c_prov - b_prov
        lines.append(
            f"| {r.name} | "
            f"{', '.join(sorted(b_prov)) or '—'} | "
            f"{', '.join(sorted(c_prov)) or '—'} | "
            f"{', '.join(sorted(missing)) or '—'} | "
            f"{', '.join(sorted(extra)) or '—'} |"
        )
    lines.append("")

    # Per-project detail — only worth showing when they diverged on status
    # or on element count, otherwise the summary table is enough.
    diverged = [
        r
        for r in have_both
        if (r.baseline.get("status") != r.claude.get("status"))
        or (_elem_total(r.baseline) != _elem_total(r.claude))
    ]
    if diverged:
        lines.append("## Divergent projects")
        lines.append("")
        for r in sorted(diverged, key=lambda x: (x.tier, x.name)):
            base = r.baseline or {}
            cl = r.claude or {}
            lines.append(f"### {r.name}")
            lines.append("")
            lines.append(
                f"- Baseline: `{base.get('status')}` — "
                f"{_elem_total(base)} elements, "
                f"{_metric(base, 'relationships')} relationships"
            )
            lines.append(
                f"- Claude:   `{cl.get('status')}` — "
                f"{_elem_total(cl)} elements, "
                f"{_metric(cl, 'relationships')} relationships"
            )
            if base.get("error"):
                lines.append(f"- Baseline error: {base['error']}")
            if cl.get("error"):
                lines.append(f"- Claude error: {cl['error']}")
            if cl.get("summary"):
                lines.append("")
                lines.append("**Claude's summary:**")
                lines.append("")
                for para in cl["summary"].split("\n\n"):
                    lines.append("> " + para.replace("\n", "\n> "))
                    lines.append("")
            lines.append("")

    return "\n".join(lines)


# ── CLI ──────────────────────────────────────────────────────────
def main() -> int:
    ap = argparse.ArgumentParser(description="Compare baseline vs Claude-driven eval")
    ap.add_argument("--stdout", action="store_true", help="write to stdout instead of compare.md")
    args = ap.parse_args()

    base = _load_dir(BASELINE_DIR)
    cl = _load_dir(CLAUDE_DIR)
    if not base and not cl:
        sys.exit("no results in results/ or results-claude/. Run the drivers first.")

    names = sorted(set(base) | set(cl))
    rows = [
        Row(
            name=n,
            tier=int((base.get(n) or cl.get(n) or {}).get("tier", 0)),
            baseline=base.get(n),
            claude=cl.get(n),
        )
        for n in names
    ]
    report = build_report(rows)

    if args.stdout:
        print(report)
    else:
        CLAUDE_DIR.mkdir(parents=True, exist_ok=True)
        COMPARE_PATH.write_text(report)
        log(f"Wrote {COMPARE_PATH}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

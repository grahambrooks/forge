#!/usr/bin/env python3
"""
Forge analyze evaluation driver.

Reads corpus.json, clones each project (shallow), runs `forge analyze`,
exports the resulting model to JSON, and scores the output against
per-project expectations. Writes one JSON result per project plus an
aggregated markdown report.

Usage:
    ./run.py                        # tier 1+2 analyze + site
    ./run.py --tier 1               # smoke tier only
    ./run.py --tier all             # include stretch tier 3
    ./run.py --only flask,gin       # specific projects
    ./run.py --no-site              # skip static-site generation
    ./run.py --forge ../forge/target/release/forge
    ./run.py report                 # regenerate report from existing results/
    ./run.py site                   # regenerate static site from existing results/
    ./run.py clean                  # remove work/ and results/

Python stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

import sitegen

# ── Paths ──────────────────────────────────────────────────────────
EVAL_DIR = Path(__file__).resolve().parent
CORPUS_PATH = EVAL_DIR / "corpus.json"
WORK_DIR = EVAL_DIR / "work"
RESULTS_DIR = EVAL_DIR / "results"
REPORT_PATH = RESULTS_DIR / "report.md"
REPO_ROOT = EVAL_DIR.parent


# ── Data model ─────────────────────────────────────────────────────
@dataclass
class Expectation:
    min_containers: int = 0
    min_relationships: int = 0
    min_elements: int = 0
    languages: list[str] = field(default_factory=list)
    scanners: list[str] = field(default_factory=list)
    max_seconds: float = 600.0


@dataclass
class Project:
    name: str
    repo: str
    ref: str
    tier: int
    description: str
    expect: Expectation

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Project":
        return cls(
            name=d["name"],
            repo=d["repo"],
            ref=d.get("ref", "main"),
            tier=int(d.get("tier", 2)),
            description=d.get("description", ""),
            expect=Expectation(**d.get("expect", {})),
        )


@dataclass
class Metrics:
    input_files: int = 0
    input_bytes: int = 0
    output_bytes: int = 0
    element_counts: dict[str, int] = field(default_factory=dict)
    relationships: int = 0
    views: int = 0
    tech_stack_entries: int = 0
    api_endpoints: int = 0
    provenance: dict[str, int] = field(default_factory=dict)


@dataclass
class Timings:
    clone_seconds: float = 0.0
    analyze_seconds: float = 0.0
    export_seconds: float = 0.0


@dataclass
class Check:
    name: str
    ok: bool
    detail: str = ""


@dataclass
class ProjectResult:
    name: str
    tier: int
    repo: str
    ref: str
    status: str  # "pass" | "fail" | "error" | "skipped"
    error: str = ""
    metrics: Metrics = field(default_factory=Metrics)
    timings: Timings = field(default_factory=Timings)
    checks: list[Check] = field(default_factory=list)


# ── Helpers ────────────────────────────────────────────────────────
def log(msg: str) -> None:
    print(msg, flush=True)


def run(
    cmd: list[str],
    *,
    cwd: Path | None = None,
    timeout: float | None = None,
    capture: bool = True,
) -> tuple[int, str, str]:
    """Run a subprocess. Returns (returncode, stdout, stderr)."""
    try:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            timeout=timeout,
            capture_output=capture,
            text=True,
            check=False,
        )
        return proc.returncode, proc.stdout or "", proc.stderr or ""
    except subprocess.TimeoutExpired as e:
        return 124, e.stdout or "", f"timeout after {timeout}s"


def human_bytes(n: int) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.0f}{unit}" if unit == "B" else f"{n:.1f}{unit}"
        n /= 1024
    return f"{n:.1f}TB"


def dir_size(path: Path) -> tuple[int, int]:
    """Return (file_count, total_bytes) for directory tree (skipping .git)."""
    files = 0
    total = 0
    for root, dirs, names in os.walk(path):
        # Skip .git to match what forge analyze ignores by default
        dirs[:] = [d for d in dirs if d != ".git"]
        for n in names:
            try:
                total += (Path(root) / n).stat().st_size
                files += 1
            except OSError:
                pass
    return files, total


def find_forge_binary(explicit: str | None) -> Path:
    """Locate the forge binary. Prefer explicit path, then release, then debug, then $PATH."""
    if explicit:
        p = Path(explicit).resolve()
        if p.exists():
            return p
        sys.exit(f"--forge {explicit} not found")
    candidates = [
        REPO_ROOT / "forge" / "target" / "release" / "forge",
        REPO_ROOT / "forge" / "target" / "debug" / "forge",
    ]
    for c in candidates:
        if c.exists():
            return c
    found = shutil.which("forge")
    if found:
        return Path(found)
    sys.exit(
        "forge binary not found. Run `cargo build --release` under forge/ "
        "or pass --forge <path>."
    )


# ── Corpus loading ─────────────────────────────────────────────────
def load_corpus() -> list[Project]:
    data = json.loads(CORPUS_PATH.read_text())
    return [Project.from_dict(p) for p in data["projects"]]


def filter_corpus(
    corpus: list[Project], *, tier: str, only: str | None
) -> list[Project]:
    if only:
        wanted = {n.strip() for n in only.split(",") if n.strip()}
        unknown = wanted - {p.name for p in corpus}
        if unknown:
            sys.exit(f"unknown project(s): {', '.join(sorted(unknown))}")
        return [p for p in corpus if p.name in wanted]
    if tier == "all":
        return corpus
    max_tier = int(tier)
    return [p for p in corpus if p.tier <= max_tier]


# ── Pipeline stages ────────────────────────────────────────────────
def clone(project: Project, *, offline: bool) -> tuple[Path, float, str | None]:
    """Shallow-clone the project if not already present. Returns (path, seconds, error)."""
    dest = WORK_DIR / project.name
    if dest.exists():
        return dest, 0.0, None
    if offline:
        return dest, 0.0, "offline: clone missing and --offline set"

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    start = time.time()
    # Try tag/ref with --depth 1 first; fall back to full-then-checkout.
    code, _, err = run(
        [
            "git",
            "clone",
            "--depth",
            "1",
            "--branch",
            project.ref,
            project.repo,
            str(dest),
        ],
        timeout=600,
    )
    if code != 0:
        # Ref may not be a branch/tag reachable by --branch; do a deeper clone.
        shutil.rmtree(dest, ignore_errors=True)
        code2, _, err2 = run(
            ["git", "clone", "--depth", "50", project.repo, str(dest)],
            timeout=600,
        )
        if code2 != 0:
            return dest, time.time() - start, f"clone failed: {err2.strip() or err.strip()}"
        code3, _, err3 = run(
            ["git", "-C", str(dest), "checkout", project.ref], timeout=120
        )
        if code3 != 0:
            return dest, time.time() - start, f"checkout {project.ref} failed: {err3.strip()}"

    return dest, time.time() - start, None


def analyze(project: Project, src: Path, forge_bin: Path) -> tuple[Path, float, str | None]:
    """Run `forge analyze` and return (output_path, seconds, error)."""
    out = RESULTS_DIR / f"{project.name}.forge"
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    log_path = RESULTS_DIR / f"{project.name}.analyze.log"

    start = time.time()
    code, stdout, stderr = run(
        [str(forge_bin), "analyze", str(src), "--out", str(out)],
        timeout=project.expect.max_seconds * 2,  # hard cap at 2x soft budget
    )
    elapsed = time.time() - start
    log_path.write_text(f"$ forge analyze {src} --out {out}\n\n{stdout}\n{stderr}")

    if code != 0:
        return out, elapsed, f"analyze exit {code}: {stderr.strip().splitlines()[-1] if stderr.strip() else 'no stderr'}"
    if not out.exists() or out.stat().st_size == 0:
        return out, elapsed, "analyze produced no output"
    return out, elapsed, None


def export_model(forge_path: Path, forge_bin: Path) -> tuple[dict | None, float, str | None]:
    """Run `forge export --format json` on the inferred model. Verifies parseability."""
    start = time.time()
    code, stdout, stderr = run(
        [str(forge_bin), "export", "--source", str(forge_path), "--format", "json"],
        timeout=60,
    )
    elapsed = time.time() - start
    if code != 0:
        return None, elapsed, f"export exit {code}: {stderr.strip().splitlines()[-1] if stderr.strip() else 'no stderr'}"
    try:
        return json.loads(stdout), elapsed, None
    except json.JSONDecodeError as e:
        return None, elapsed, f"export produced invalid JSON: {e}"


def compute_metrics(
    src: Path, forge_path: Path, model: dict
) -> Metrics:
    file_count, byte_count = dir_size(src)
    elements = model.get("elements", [])
    kinds = Counter(e.get("kind", "Unknown") for e in elements)
    provenance: Counter[str] = Counter()
    for e in elements:
        for tag in e.get("tags") or []:
            if isinstance(tag, str) and tag.startswith("inferred:"):
                provenance[tag[len("inferred:") :]] += 1
    tech_entries = sum(
        len(c.get("entries") or []) for c in (model.get("tech-stack") or [])
    )
    # api_endpoints: not exported directly in JSON; derive 0 (extensible later).
    return Metrics(
        input_files=file_count,
        input_bytes=byte_count,
        output_bytes=forge_path.stat().st_size if forge_path.exists() else 0,
        element_counts=dict(kinds),
        relationships=len(model.get("relationships") or []),
        views=len(model.get("views") or []),
        tech_stack_entries=tech_entries,
        api_endpoints=0,
        provenance=dict(provenance),
    )


def evaluate(project: Project, metrics: Metrics, analyze_seconds: float) -> list[Check]:
    checks: list[Check] = []
    exp = project.expect

    container_count = metrics.element_counts.get("Container", 0)
    checks.append(
        Check(
            "min_containers",
            container_count >= exp.min_containers,
            f"found {container_count}, expected ≥ {exp.min_containers}",
        )
    )
    if exp.min_elements:
        total_elements = sum(metrics.element_counts.values())
        checks.append(
            Check(
                "min_elements",
                total_elements >= exp.min_elements,
                f"found {total_elements}, expected ≥ {exp.min_elements}",
            )
        )
    if exp.min_relationships:
        checks.append(
            Check(
                "min_relationships",
                metrics.relationships >= exp.min_relationships,
                f"found {metrics.relationships}, expected ≥ {exp.min_relationships}",
            )
        )
    for scanner in exp.scanners:
        count = metrics.provenance.get(scanner, 0)
        checks.append(
            Check(
                f"scanner:{scanner}",
                count > 0,
                f"{count} element(s) tagged inferred:{scanner}",
            )
        )
    checks.append(
        Check(
            "analyze_budget",
            analyze_seconds <= exp.max_seconds,
            f"{analyze_seconds:.1f}s vs budget {exp.max_seconds:.0f}s",
        )
    )
    return checks


def process(project: Project, forge_bin: Path, *, offline: bool) -> ProjectResult:
    log(f"→ {project.name} (tier {project.tier}) …")
    res = ProjectResult(
        name=project.name,
        tier=project.tier,
        repo=project.repo,
        ref=project.ref,
        status="error",
    )

    src, clone_secs, err = clone(project, offline=offline)
    res.timings.clone_seconds = clone_secs
    if err:
        res.error = err
        log(f"  ✗ {err}")
        return res

    forge_path, analyze_secs, err = analyze(project, src, forge_bin)
    res.timings.analyze_seconds = analyze_secs
    if err:
        res.error = err
        log(f"  ✗ {err}")
        return res

    model, export_secs, err = export_model(forge_path, forge_bin)
    res.timings.export_seconds = export_secs
    if err or model is None:
        res.error = err or "no model"
        log(f"  ✗ {res.error}")
        return res

    # Persist the exported JSON for later drill-down.
    (RESULTS_DIR / f"{project.name}.model.json").write_text(json.dumps(model, indent=2))

    res.metrics = compute_metrics(src, forge_path, model)
    res.checks = evaluate(project, res.metrics, analyze_secs)
    res.status = "pass" if all(c.ok for c in res.checks) else "fail"

    c_total = len(res.checks)
    c_ok = sum(1 for c in res.checks if c.ok)
    log(
        f"  {'✓' if res.status == 'pass' else '○'} {res.status} "
        f"({c_ok}/{c_total} checks, {analyze_secs:.1f}s, "
        f"{sum(res.metrics.element_counts.values())} elements)"
    )
    return res


# ── Reporting ──────────────────────────────────────────────────────
def write_result(res: ProjectResult) -> None:
    path = RESULTS_DIR / f"{res.name}.json"
    path.write_text(json.dumps(asdict(res), indent=2, default=str))


def load_results() -> list[ProjectResult]:
    out: list[ProjectResult] = []
    if not RESULTS_DIR.exists():
        return out
    for p in sorted(RESULTS_DIR.glob("*.json")):
        if p.name == "report.json":
            continue
        try:
            d = json.loads(p.read_text())
        except json.JSONDecodeError:
            continue
        # Skip files that aren't project result envelopes
        if not isinstance(d, dict) or "status" not in d or "checks" not in d:
            continue
        res = ProjectResult(
            name=d["name"],
            tier=d.get("tier", 0),
            repo=d.get("repo", ""),
            ref=d.get("ref", ""),
            status=d["status"],
            error=d.get("error", ""),
            metrics=Metrics(**d.get("metrics", {})),
            timings=Timings(**d.get("timings", {})),
            checks=[Check(**c) for c in d.get("checks", [])],
        )
        out.append(res)
    return out


def write_report(results: list[ProjectResult]) -> None:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    lines.append("# Forge analyze evaluation report")
    lines.append("")
    total = len(results)
    passed = sum(1 for r in results if r.status == "pass")
    failed = sum(1 for r in results if r.status == "fail")
    errored = sum(1 for r in results if r.status == "error")
    lines.append(f"**{passed}/{total} passed** ({failed} soft-fail, {errored} error)")
    lines.append("")

    # Summary table
    lines.append("## Summary")
    lines.append("")
    lines.append(
        "| Project | Tier | Status | Elements | Relations | Views | Time | Scanners |"
    )
    lines.append(
        "| --- | --- | --- | --- | --- | --- | --- | --- |"
    )
    for r in sorted(results, key=lambda x: (x.tier, x.name)):
        elem_total = sum(r.metrics.element_counts.values())
        scanners = ",".join(sorted(r.metrics.provenance.keys())) or "—"
        status_icon = {"pass": "✓", "fail": "○", "error": "✗"}.get(r.status, "?")
        lines.append(
            f"| {r.name} | {r.tier} | {status_icon} {r.status} | {elem_total} | "
            f"{r.metrics.relationships} | {r.metrics.views} | "
            f"{r.timings.analyze_seconds:.1f}s | {scanners} |"
        )
    lines.append("")

    # Per-project detail
    lines.append("## Details")
    lines.append("")
    for r in sorted(results, key=lambda x: (x.tier, x.name)):
        lines.append(f"### {r.name} — tier {r.tier} — {r.status}")
        lines.append("")
        lines.append(f"- Repo: `{r.repo}` @ `{r.ref}`")
        if r.error:
            lines.append(f"- **Error**: {r.error}")
        lines.append(
            f"- Input: {r.metrics.input_files} files, "
            f"{human_bytes(r.metrics.input_bytes)}"
        )
        lines.append(
            f"- Output: {human_bytes(r.metrics.output_bytes)} .forge"
        )
        lines.append(
            f"- Timings: clone {r.timings.clone_seconds:.1f}s, "
            f"analyze {r.timings.analyze_seconds:.1f}s, "
            f"export {r.timings.export_seconds:.1f}s"
        )
        if r.metrics.element_counts:
            by_kind = ", ".join(
                f"{k}: {v}" for k, v in sorted(r.metrics.element_counts.items())
            )
            lines.append(f"- Elements: {by_kind}")
        if r.metrics.provenance:
            prov = ", ".join(
                f"{k}: {v}" for k, v in sorted(r.metrics.provenance.items())
            )
            lines.append(f"- Provenance: {prov}")
        lines.append(f"- Views: {r.metrics.views}")
        if r.checks:
            lines.append("- Checks:")
            for c in r.checks:
                mark = "✓" if c.ok else "✗"
                lines.append(f"    - {mark} `{c.name}` — {c.detail}")
        lines.append("")

    REPORT_PATH.write_text("\n".join(lines))
    log(f"\nWrote {REPORT_PATH}")




# ── CLI ────────────────────────────────────────────────────────────
def main() -> int:
    ap = argparse.ArgumentParser(description="Forge analyze evaluation framework")
    ap.add_argument(
        "command",
        nargs="?",
        default="run",
        choices=["run", "report", "site", "clean"],
    )
    ap.add_argument("--tier", default="2", help="max tier to run (1, 2, 3, all); default 2")
    ap.add_argument("--only", help="comma-separated project names (overrides --tier)")
    ap.add_argument("--forge", help="path to forge binary (default: auto-detect)")
    ap.add_argument("--offline", action="store_true", help="don't clone; use existing work/")
    ap.add_argument(
        "--no-site",
        action="store_true",
        help="skip static-site generation after `run`",
    )
    ap.add_argument(
        "--keep-going",
        action="store_true",
        help="continue past errors (default)",
        default=True,
    )
    args = ap.parse_args()

    if args.command == "clean":
        for p in (WORK_DIR, RESULTS_DIR):
            if p.exists():
                shutil.rmtree(p)
                log(f"removed {p}")
        return 0

    if args.command == "report":
        results = load_results()
        if not results:
            log("no results found. Run `./run.py` first.")
            return 1
        write_report(results)
        return 0

    if args.command == "site":
        forge_bin = find_forge_binary(args.forge)
        results = load_results()
        claude_dir = EVAL_DIR / "results-claude"
        if not results and not claude_dir.exists():
            log("no results found. Run `./run.py` first.")
            return 1
        # sitegen reads both baseline and claude results from disk, so the
        # --only / --tier restrictions apply to the metric collection phase
        # (run/report), not to the rendered site: the site always mirrors
        # whichever .json files are in results/ and results-claude/.
        sitegen.build_site(forge_bin=forge_bin)
        return 0

    forge_bin = find_forge_binary(args.forge)
    log(f"forge: {forge_bin}")

    corpus = filter_corpus(load_corpus(), tier=args.tier, only=args.only)
    log(f"running {len(corpus)} project(s)")

    results: list[ProjectResult] = []
    for project in corpus:
        res = process(project, forge_bin, offline=args.offline)
        write_result(res)
        results.append(res)

    write_report(results)

    if not args.no_site:
        log("\nbuilding static site…")
        sitegen.build_site(forge_bin=forge_bin)

    hard_errors = sum(1 for r in results if r.status == "error")
    return 1 if hard_errors else 0


if __name__ == "__main__":
    sys.exit(main())

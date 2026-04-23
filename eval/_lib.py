"""Shared helpers for the Forge evaluation drivers.

Both `run.py` (baseline `forge analyze`) and `run_claude.py` (Claude-driven
via forge-architect plugin) depend on the same corpus loading, clone cache,
and metric computation. Keeping them here avoids drift.

Python stdlib only.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

# ── Paths (shared by both drivers) ───────────────────────────────
EVAL_DIR = Path(__file__).resolve().parent
CORPUS_PATH = EVAL_DIR / "corpus.json"
WORK_DIR = EVAL_DIR / "work"
REPO_ROOT = EVAL_DIR.parent


# ── Data model ───────────────────────────────────────────────────
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
class Check:
    name: str
    ok: bool
    detail: str = ""


# ── Logging ──────────────────────────────────────────────────────
def log(msg: str) -> None:
    print(msg, flush=True)


# ── Process helpers ──────────────────────────────────────────────
def run(
    cmd: list[str],
    *,
    cwd: Path | None = None,
    timeout: float | None = None,
    capture: bool = True,
    env: dict[str, str] | None = None,
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
            env=env,
        )
        return proc.returncode, proc.stdout or "", proc.stderr or ""
    except subprocess.TimeoutExpired as e:
        return 124, e.stdout or "", f"timeout after {timeout}s"


# ── Binary discovery ─────────────────────────────────────────────
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


def find_claude_binary(explicit: str | None) -> Path:
    """Locate the claude CLI. Fails loudly if missing — there is no sensible fallback."""
    if explicit:
        p = Path(explicit).resolve()
        if p.exists():
            return p
        sys.exit(f"--claude {explicit} not found")
    found = shutil.which("claude")
    if not found:
        sys.exit(
            "`claude` CLI not found on PATH. Install Claude Code, or pass "
            "--claude /path/to/claude."
        )
    return Path(found)


# ── Corpus ───────────────────────────────────────────────────────
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


# ── Clone cache (shared between drivers) ────────────────────────
def clone(project: Project, *, offline: bool) -> tuple[Path, float, str | None]:
    """Shallow-clone the project into `work/<name>/` if not already present.

    Both drivers hit the same cache — the first one to run a project pays the
    clone cost, the second reuses it. Returns (path, seconds, error).
    """
    dest = WORK_DIR / project.name
    if dest.exists():
        return dest, 0.0, None
    if offline:
        return dest, 0.0, "offline: clone missing and --offline set"

    WORK_DIR.mkdir(parents=True, exist_ok=True)
    start = time.time()
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
        shutil.rmtree(dest, ignore_errors=True)
        code2, _, err2 = run(
            ["git", "clone", "--depth", "50", project.repo, str(dest)],
            timeout=600,
        )
        if code2 != 0:
            return (
                dest,
                time.time() - start,
                f"clone failed: {err2.strip() or err.strip()}",
            )
        code3, _, err3 = run(
            ["git", "-C", str(dest), "checkout", project.ref], timeout=120
        )
        if code3 != 0:
            return (
                dest,
                time.time() - start,
                f"checkout {project.ref} failed: {err3.strip()}",
            )

    return dest, time.time() - start, None


# ── Metrics ──────────────────────────────────────────────────────
def dir_size(path: Path) -> tuple[int, int]:
    """Return (file_count, total_bytes) for a tree, skipping .git/."""
    files = 0
    total = 0
    for root, dirs, names in os.walk(path):
        dirs[:] = [d for d in dirs if d != ".git"]
        for n in names:
            try:
                total += (Path(root) / n).stat().st_size
                files += 1
            except OSError:
                pass
    return files, total


def export_model(
    forge_path: Path, forge_bin: Path
) -> tuple[dict | None, float, str | None]:
    """Run `forge export --format json` and return the parsed model."""
    start = time.time()
    code, stdout, stderr = run(
        [str(forge_bin), "export", "--source", str(forge_path), "--format", "json"],
        timeout=60,
    )
    elapsed = time.time() - start
    if code != 0:
        last = stderr.strip().splitlines()[-1] if stderr.strip() else "no stderr"
        return None, elapsed, f"export exit {code}: {last}"
    try:
        return json.loads(stdout), elapsed, None
    except json.JSONDecodeError as e:
        return None, elapsed, f"export produced invalid JSON: {e}"


def compute_metrics(src: Path, forge_path: Path, model: dict) -> Metrics:
    file_count, byte_count = dir_size(src)
    elements = model.get("elements", [])
    kinds: Counter[str] = Counter(e.get("kind", "Unknown") for e in elements)
    provenance: Counter[str] = Counter()
    for e in elements:
        for tag in e.get("tags") or []:
            if isinstance(tag, str) and tag.startswith("inferred:"):
                provenance[tag[len("inferred:"):]] += 1
    tech_entries = sum(
        len(c.get("entries") or []) for c in (model.get("tech-stack") or [])
    )
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


def check_violations(
    forge_path: Path, forge_bin: Path, *, severity: str = "warning"
) -> tuple[list[dict], str | None]:
    """Run `forge check --format json` against a .forge file. Returns the
    parsed violation list (empty if none) and an optional error string."""
    code, stdout, stderr = run(
        [
            str(forge_bin),
            "check",
            "--source",
            str(forge_path),
            "--severity",
            severity,
            "--format",
            "json",
        ],
        timeout=60,
    )
    # `forge check` exits 1 when error-severity violations exist; that's fine —
    # we only care about the JSON payload.
    if code not in (0, 1):
        last = stderr.strip().splitlines()[-1] if stderr.strip() else "no stderr"
        return [], f"check exit {code}: {last}"
    try:
        return json.loads(stdout or "[]"), None
    except json.JSONDecodeError as e:
        return [], f"check produced invalid JSON: {e}"


# ── Formatting ───────────────────────────────────────────────────
def human_bytes(n: int) -> str:
    size = float(n)
    for unit in ("B", "KB", "MB", "GB"):
        if size < 1024:
            return f"{size:.0f}{unit}" if unit == "B" else f"{size:.1f}{unit}"
        size /= 1024
    return f"{size:.1f}TB"

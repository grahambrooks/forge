#!/usr/bin/env python3
"""
Companion Forge evaluation driver — runs Claude Code (headless, `claude -p`)
with the forge-architect plugin over the same corpus as run.py, writes the
resulting .forge file, and captures the transcript.

The prompt steers Claude to use the `model-repository` skill:

  1. Call `forge_analyze` via the MCP on the cloned project.
  2. Inspect the result with `forge_overview` and `forge_check`.
  3. Save the final .forge to the designated output path.

We then parse that .forge the same way the baseline does (`forge export`)
and compute identical metrics, so results are directly comparable.

Usage:
    ./run_claude.py                       # tier 1, up to $0.50/project
    ./run_claude.py --tier 2              # broader
    ./run_claude.py --only flask,gin      # specific projects
    ./run_claude.py --budget 1.00         # per-project dollar cap
    ./run_claude.py --model opus          # or a full model id
    ./run_claude.py report                # regenerate markdown from cache
    ./run_claude.py clean                 # remove results-claude/

Prerequisites:
  - `forge` binary on PATH (or pass --forge)
  - `claude` CLI on PATH (or pass --claude)
  - The forge-architect plugin is passed inline via --plugin-dir; no prior
    symlinking required.

Python stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

import sitegen
from _lib import (
    EVAL_DIR,
    REPO_ROOT,
    Check,
    Metrics,
    Project,
    check_violations,
    clone,
    compute_metrics,
    export_model,
    filter_corpus,
    find_claude_binary,
    find_forge_binary,
    human_bytes,
    load_corpus,
    log,
    run,
)

# ── Paths specific to the Claude driver ──────────────────────────
RESULTS_DIR = EVAL_DIR / "results-claude"
REPORT_PATH = RESULTS_DIR / "report.md"
PLUGIN_DIR = REPO_ROOT / "integrations" / "claude-plugin" / "forge-architect"

# Maximum wall-clock per project. The Claude loop can legitimately take a
# couple of minutes for tier-2 repos; the budget flag is the real limiter.
DEFAULT_TIMEOUT_SECS = 600

# Prompt that pushes Claude into the model-repository skill and pins down
# the output location. Kept deliberately short — the skill already contains
# the playbook. {out} is substituted per project.
PROMPT_TEMPLATE = """\
You are evaluating the `forge-architect` plugin. Produce a Forge architecture \
model for the repository rooted at the current working directory.

The `forge` MCP server is already configured for this session. Its tools are \
`mcp__forge__forge_analyze`, `mcp__forge__forge_overview`, \
`mcp__forge__forge_check`, etc. Do not run any prerequisite checks — call \
the tools directly.

Steps:
1. Call `mcp__forge__forge_analyze` with `path="."` and `out="{out}"`. Do \
   not wrap this in any setup commands. The MCP server already has forge \
   loaded; this call runs the analyzer and writes the file.
2. Call `mcp__forge__forge_overview` to confirm what was detected.
3. Call `mcp__forge__forge_check` with severity "info" and list the top \
   violations (up to five).
4. Briefly (≤5 bullets) describe what the model says about the repo: \
   languages, services, pipelines, any obvious gaps.

Do not hand-edit the .forge file for this run — we are measuring the model \
the analyzer plus the MCP tooling produces. End with the single line \
`DONE {out}`.
"""


# ── Data model ───────────────────────────────────────────────────
@dataclass
class Usage:
    input_tokens: int = 0
    output_tokens: int = 0
    cache_read_tokens: int = 0
    cache_write_tokens: int = 0
    cost_usd: float = 0.0
    turns: int = 0


@dataclass
class Timings:
    clone_seconds: float = 0.0
    claude_seconds: float = 0.0
    export_seconds: float = 0.0


@dataclass
class ClaudeResult:
    name: str
    tier: int
    repo: str
    ref: str
    status: str  # "pass" | "fail" | "error"
    error: str = ""
    metrics: Metrics = field(default_factory=Metrics)
    timings: Timings = field(default_factory=Timings)
    usage: Usage = field(default_factory=Usage)
    checks: list[Check] = field(default_factory=list)
    violations: list[dict] = field(default_factory=list)
    model: str = ""
    summary: str = ""


# ── Claude invocation ────────────────────────────────────────────
def run_claude(
    project: Project,
    src: Path,
    out_forge: Path,
    *,
    claude_bin: Path,
    forge_bin: Path,
    model: str,
    budget_usd: float,
    timeout_secs: float,
    transcript_path: Path,
) -> tuple[dict | None, float, str | None]:
    """Invoke `claude -p` in the project directory and capture the JSON transcript.

    Returns (parsed-transcript-dict, seconds, error).
    """
    prompt = PROMPT_TEMPLATE.format(out=str(out_forge))

    # Register the forge MCP server inline with an absolute path. Relying on
    # the plugin.json declaration (`command: "forge"`) fails when forge isn't
    # on PATH in Claude's subprocess — a common case when building from source.
    mcp_config = json.dumps(
        {"mcpServers": {"forge": {"command": str(forge_bin), "args": ["mcp"]}}}
    )

    cmd = [
        str(claude_bin),
        "-p",
        prompt,
        "--output-format",
        "json",
        "--model",
        model,
        "--permission-mode",
        "bypassPermissions",
        "--dangerously-skip-permissions",
        "--max-budget-usd",
        f"{budget_usd:.2f}",
        "--plugin-dir",
        str(PLUGIN_DIR),
        "--mcp-config",
        mcp_config,
        "--add-dir",
        str(RESULTS_DIR),
        "--setting-sources",
        "user",
        "--no-session-persistence",
        # Restrict tool surface: Claude only needs the forge MCP plus read-
        # only exploration on the repo, and a narrow `forge` CLI fallback
        # for the case where the MCP fails to register. Other Bash is denied
        # so the eval can't mutate the clone cache.
        "--allowedTools",
        "Read Glob Grep "
        "Bash(ls:*) Bash(cat:*) Bash(head:*) Bash(forge:*) "
        "mcp__forge__forge_analyze mcp__forge__forge_overview "
        "mcp__forge__forge_check mcp__forge__forge_list_views "
        "mcp__forge__forge_query mcp__forge__forge_search "
        "mcp__forge__forge_element_detail mcp__forge__forge_validate",
    ]

    # Belt-and-braces: also put the forge binary's directory on PATH for the
    # subprocess. If anything else inside the session tries `forge` by name
    # (e.g. a Bash command Claude writes), it will resolve.
    env = dict(os.environ)
    env["PATH"] = f"{forge_bin.parent}{os.pathsep}{env.get('PATH', '')}"

    start = time.time()
    code, stdout, stderr = run(cmd, cwd=src, timeout=timeout_secs, env=env)
    elapsed = time.time() - start

    # Always persist the raw transcript — it's how a human reviews what the
    # model actually did.
    transcript_path.write_text(stdout or stderr or "")

    if code != 0 and not stdout:
        last = stderr.strip().splitlines()[-1] if stderr.strip() else "no stderr"
        return None, elapsed, f"claude exit {code}: {last}"

    try:
        parsed = json.loads(stdout)
    except json.JSONDecodeError as e:
        return None, elapsed, f"claude output was not JSON: {e}"

    # `claude -p --output-format json` returns either a success envelope
    # (`{"result": "...", "usage": {...}}`) or an error envelope. Treat any
    # envelope as useful for forensics but flag error subtypes.
    if parsed.get("is_error"):
        msg = parsed.get("result") or parsed.get("error") or "unknown claude error"
        return parsed, elapsed, f"claude reported error: {msg}"

    # The MCP tool must actually have written the file. If Claude responded
    # without producing the artifact, that's a real miss worth flagging.
    if not out_forge.exists() or out_forge.stat().st_size == 0:
        return (
            parsed,
            elapsed,
            "claude finished without producing a .forge file",
        )

    # Also run a quick syntactic check via the forge binary so we don't
    # compute metrics on a corrupt file.
    code2, _, stderr2 = run(
        [str(forge_bin), "check", "--source", str(out_forge), "--severity", "info"],
        timeout=30,
    )
    if code2 not in (0, 1):
        last = stderr2.strip().splitlines()[-1] if stderr2.strip() else "no stderr"
        return parsed, elapsed, f"produced .forge failed parse: {last}"

    return parsed, elapsed, None


def extract_usage(transcript: dict) -> Usage:
    """Pull token counts, cost, and turn count out of a `claude -p --output-format json` envelope."""
    usage_raw = transcript.get("usage") or {}
    return Usage(
        input_tokens=int(usage_raw.get("input_tokens", 0)),
        output_tokens=int(usage_raw.get("output_tokens", 0)),
        cache_read_tokens=int(usage_raw.get("cache_read_input_tokens", 0)),
        cache_write_tokens=int(usage_raw.get("cache_creation_input_tokens", 0)),
        cost_usd=float(transcript.get("total_cost_usd", 0.0)),
        turns=int(transcript.get("num_turns", 0)),
    )


def extract_summary(transcript: dict) -> str:
    """The final assistant text. Handy for the report's per-project section."""
    text = transcript.get("result") or ""
    if isinstance(text, str):
        # Trim the `DONE <path>` sentinel line — it's noise in a report.
        lines = [line for line in text.splitlines() if not line.strip().startswith("DONE ")]
        return "\n".join(lines).strip()
    return ""


# ── Evaluation ───────────────────────────────────────────────────
def evaluate(project: Project, metrics: Metrics, claude_seconds: float) -> list[Check]:
    """Identical check shape to run.py so the comparison tool can align them."""
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
    # Claude's wall-clock is dominated by model latency, not analysis — use
    # a separate generous budget (5× baseline) rather than the baseline
    # `max_seconds`. The real cost cap is --budget, enforced by Claude.
    budget = exp.max_seconds * 5
    checks.append(
        Check(
            "claude_budget",
            claude_seconds <= budget,
            f"{claude_seconds:.1f}s vs budget {budget:.0f}s (5× baseline)",
        )
    )
    return checks


def process(
    project: Project,
    *,
    claude_bin: Path,
    forge_bin: Path,
    model: str,
    budget_usd: float,
    timeout_secs: float,
    offline: bool,
) -> ClaudeResult:
    log(f"→ {project.name} (tier {project.tier}) …")
    res = ClaudeResult(
        name=project.name,
        tier=project.tier,
        repo=project.repo,
        ref=project.ref,
        status="error",
        model=model,
    )

    src, clone_secs, err = clone(project, offline=offline)
    res.timings.clone_seconds = clone_secs
    if err:
        res.error = err
        log(f"  ✗ {err}")
        return res

    out_forge = RESULTS_DIR / f"{project.name}.forge"
    transcript_path = RESULTS_DIR / f"{project.name}.transcript.json"

    parsed, claude_secs, err = run_claude(
        project,
        src,
        out_forge,
        claude_bin=claude_bin,
        forge_bin=forge_bin,
        model=model,
        budget_usd=budget_usd,
        timeout_secs=timeout_secs,
        transcript_path=transcript_path,
    )
    res.timings.claude_seconds = claude_secs
    if parsed is not None:
        res.usage = extract_usage(parsed)
        res.summary = extract_summary(parsed)
    if err:
        res.error = err
        log(f"  ✗ {err}")
        return res

    model_dict, export_secs, err = export_model(out_forge, forge_bin)
    res.timings.export_seconds = export_secs
    if err or model_dict is None:
        res.error = err or "no model"
        log(f"  ✗ {res.error}")
        return res

    (RESULTS_DIR / f"{project.name}.model.json").write_text(
        json.dumps(model_dict, indent=2)
    )

    res.metrics = compute_metrics(src, out_forge, model_dict)
    res.violations, _ = check_violations(out_forge, forge_bin, severity="info")
    res.checks = evaluate(project, res.metrics, claude_secs)
    res.status = "pass" if all(c.ok for c in res.checks) else "fail"

    c_ok = sum(1 for c in res.checks if c.ok)
    log(
        f"  {'✓' if res.status == 'pass' else '○'} {res.status} "
        f"({c_ok}/{len(res.checks)} checks, {claude_secs:.1f}s, "
        f"${res.usage.cost_usd:.3f}, "
        f"{sum(res.metrics.element_counts.values())} elements, "
        f"{res.usage.turns} turns)"
    )
    return res


# ── Reporting ────────────────────────────────────────────────────
def write_result(res: ClaudeResult) -> None:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    (RESULTS_DIR / f"{res.name}.json").write_text(
        json.dumps(asdict(res), indent=2, default=str)
    )


def load_results() -> list[ClaudeResult]:
    out: list[ClaudeResult] = []
    if not RESULTS_DIR.exists():
        return out
    for p in sorted(RESULTS_DIR.glob("*.json")):
        if "transcript" in p.name or "model" in p.name:
            continue
        try:
            d = json.loads(p.read_text())
        except json.JSONDecodeError:
            continue
        if not isinstance(d, dict) or "status" not in d or "checks" not in d:
            continue
        out.append(
            ClaudeResult(
                name=d["name"],
                tier=d.get("tier", 0),
                repo=d.get("repo", ""),
                ref=d.get("ref", ""),
                status=d["status"],
                error=d.get("error", ""),
                metrics=Metrics(**d.get("metrics", {})),
                timings=Timings(**d.get("timings", {})),
                usage=Usage(**d.get("usage", {})),
                checks=[Check(**c) for c in d.get("checks", [])],
                violations=d.get("violations", []),
                model=d.get("model", ""),
                summary=d.get("summary", ""),
            )
        )
    return out


def write_report(results: list[ClaudeResult]) -> None:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    lines.append("# Forge eval — Claude-driven (forge-architect)")
    lines.append("")
    total = len(results)
    passed = sum(1 for r in results if r.status == "pass")
    failed = sum(1 for r in results if r.status == "fail")
    errored = sum(1 for r in results if r.status == "error")
    total_cost = sum(r.usage.cost_usd for r in results)
    total_turns = sum(r.usage.turns for r in results)
    lines.append(
        f"**{passed}/{total} passed** ({failed} soft-fail, {errored} error)  ·  "
        f"${total_cost:.2f} total  ·  {total_turns} turns"
    )
    lines.append("")

    lines.append("## Summary")
    lines.append("")
    lines.append(
        "| Project | Tier | Status | Elements | Relations | Views | Turns | Cost | Wall |"
    )
    lines.append("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for r in sorted(results, key=lambda x: (x.tier, x.name)):
        elem_total = sum(r.metrics.element_counts.values())
        status_icon = {"pass": "✓", "fail": "○", "error": "✗"}.get(r.status, "?")
        lines.append(
            f"| {r.name} | {r.tier} | {status_icon} {r.status} | {elem_total} | "
            f"{r.metrics.relationships} | {r.metrics.views} | {r.usage.turns} | "
            f"${r.usage.cost_usd:.3f} | {r.timings.claude_seconds:.1f}s |"
        )
    lines.append("")

    lines.append("## Details")
    lines.append("")
    for r in sorted(results, key=lambda x: (x.tier, x.name)):
        lines.append(f"### {r.name} — tier {r.tier} — {r.status}")
        lines.append("")
        lines.append(f"- Repo: `{r.repo}` @ `{r.ref}`")
        lines.append(f"- Model: `{r.model}`")
        if r.error:
            lines.append(f"- **Error**: {r.error}")
        lines.append(
            f"- Input: {r.metrics.input_files} files, "
            f"{human_bytes(r.metrics.input_bytes)}"
        )
        lines.append(f"- Output: {human_bytes(r.metrics.output_bytes)} .forge")
        lines.append(
            f"- Timings: clone {r.timings.clone_seconds:.1f}s, "
            f"claude {r.timings.claude_seconds:.1f}s, "
            f"export {r.timings.export_seconds:.1f}s"
        )
        lines.append(
            f"- Usage: {r.usage.turns} turns, "
            f"{r.usage.input_tokens}+{r.usage.output_tokens} tokens "
            f"(cache {r.usage.cache_read_tokens}r/{r.usage.cache_write_tokens}w), "
            f"${r.usage.cost_usd:.3f}"
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
        if r.violations:
            lines.append(
                f"- Violations (info+): {len(r.violations)}"
            )
            by_rule: dict[str, int] = {}
            for v in r.violations:
                by_rule[v.get("rule", "?")] = by_rule.get(v.get("rule", "?"), 0) + 1
            lines.append(
                "    - " + ", ".join(f"`{k}`: {v}" for k, v in sorted(by_rule.items()))
            )
        if r.summary:
            lines.append("")
            lines.append("**Claude's summary:**")
            lines.append("")
            for para in r.summary.split("\n\n"):
                lines.append("> " + para.replace("\n", "\n> "))
                lines.append("")
        lines.append("")

    REPORT_PATH.write_text("\n".join(lines))
    log(f"\nWrote {REPORT_PATH}")


# ── CLI ──────────────────────────────────────────────────────────
def main() -> int:
    ap = argparse.ArgumentParser(
        description="Forge evaluation — Claude-driven via forge-architect plugin"
    )
    ap.add_argument(
        "command",
        nargs="?",
        default="run",
        choices=["run", "report", "clean"],
    )
    ap.add_argument("--tier", default="1", help="max tier to run (1, 2, 3, all); default 1 (conservative due to API cost)")
    ap.add_argument("--only", help="comma-separated project names (overrides --tier)")
    ap.add_argument("--forge", help="path to forge binary (default: auto-detect)")
    ap.add_argument("--claude", help="path to claude CLI (default: $PATH)")
    ap.add_argument(
        "--model",
        default="sonnet",
        help="Claude model alias or id (default: sonnet)",
    )
    ap.add_argument(
        "--budget",
        type=float,
        default=0.50,
        help="max USD per project (passed to --max-budget-usd); default 0.50",
    )
    ap.add_argument(
        "--timeout",
        type=float,
        default=DEFAULT_TIMEOUT_SECS,
        help=f"wall-clock per project in seconds; default {DEFAULT_TIMEOUT_SECS}",
    )
    ap.add_argument("--offline", action="store_true", help="don't clone; use existing work/")
    ap.add_argument(
        "--no-site",
        action="store_true",
        help="skip static-site regeneration after `run`",
    )
    args = ap.parse_args()

    if args.command == "clean":
        if RESULTS_DIR.exists():
            shutil.rmtree(RESULTS_DIR)
            log(f"removed {RESULTS_DIR}")
        return 0

    if args.command == "report":
        results = load_results()
        if not results:
            log(f"no results in {RESULTS_DIR}. Run `./run_claude.py` first.")
            return 1
        write_report(results)
        return 0

    if not PLUGIN_DIR.exists():
        sys.exit(
            f"forge-architect plugin not found at {PLUGIN_DIR}. "
            "Are you running from a repo without the plugin scaffolded?"
        )

    claude_bin = find_claude_binary(args.claude)
    forge_bin = find_forge_binary(args.forge)
    log(f"claude: {claude_bin}")
    log(f"forge:  {forge_bin}")
    log(f"plugin: {PLUGIN_DIR}")
    log(f"model:  {args.model}")
    log(f"budget: ${args.budget:.2f} per project, ${0 if not args.tier else '?'} total")

    corpus = filter_corpus(load_corpus(), tier=args.tier, only=args.only)
    log(f"running {len(corpus)} project(s)")

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    results: list[ClaudeResult] = []
    for project in corpus:
        res = process(
            project,
            claude_bin=claude_bin,
            forge_bin=forge_bin,
            model=args.model,
            budget_usd=args.budget,
            timeout_secs=args.timeout,
            offline=args.offline,
        )
        write_result(res)
        results.append(res)

    write_report(results)

    total_cost = sum(r.usage.cost_usd for r in results)
    log(f"\ntotal spend: ${total_cost:.2f}")

    if not args.no_site:
        log("\nrebuilding combined site…")
        sitegen.build_site(forge_bin=forge_bin)

    hard_errors = sum(1 for r in results if r.status == "error")
    return 1 if hard_errors else 0


if __name__ == "__main__":
    sys.exit(main())

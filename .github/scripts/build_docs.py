#!/usr/bin/env python3
"""
Assemble the GitHub Pages site.

Renders the project's top-level markdown docs into standalone HTML pages
under a shared layout, and writes the landing `index.html` that links out
to the demo site (`/demo/`) and the evaluation site (`/eval/`) produced
separately by the Pages workflow.

Usage:
    python3 build_docs.py <out_site_root>
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

import markdown

REPO_ROOT = Path(__file__).resolve().parents[2]


# ── Page config ────────────────────────────────────────────────────
@dataclass
class Page:
    slug: str  # URL path segment under the site root ("" for index)
    title: str
    source: Path  # markdown file path relative to REPO_ROOT
    nav_label: str
    hide_from_nav: bool = False


PAGES: list[Page] = [
    Page("readme", "Readme", REPO_ROOT / "README.md", "Readme"),
    Page("design", "Design", REPO_ROOT / "DESIGN.md", "Design"),
    Page("editors", "Editor setup", REPO_ROOT / "forge" / "EDITORS.md", "Editors"),
    Page(
        "publishing",
        "Publishing guide",
        REPO_ROOT / "forge" / "PUBLISHING.md",
        "Publishing",
    ),
    Page(
        "eval-readme",
        "Evaluation framework",
        REPO_ROOT / "eval" / "README.md",
        "Eval framework",
    ),
]

# Each entry: (label, href). Hrefs starting with "." are resolved relative
# to the current page's depth; absolute URLs (http://…) pass through.
EXTERNAL_NAV = [
    ("Demo", "./demo/"),
    ("Evaluation", "./eval/"),
    ("GitHub", "https://github.com/grahambrooks/forge"),
]


# ── Stylesheet ─────────────────────────────────────────────────────
DOCS_CSS = """\
:root {
    --fg: #1f2430;
    --fg-muted: #586069;
    --bg: #ffffff;
    --bg-alt: #f6f8fa;
    --border: #d0d7de;
    --accent: #0969da;
    --accent-weak: #ddf4ff;
    --code-bg: #f6f8fa;
    --code-border: #d0d7de;
    --shadow: 0 1px 0 rgba(27, 31, 36, 0.04), 0 1px 3px rgba(27, 31, 36, 0.06);
}
@media (prefers-color-scheme: dark) {
    :root {
        --fg: #e6edf3;
        --fg-muted: #8b949e;
        --bg: #0d1117;
        --bg-alt: #161b22;
        --border: #30363d;
        --accent: #58a6ff;
        --accent-weak: #132e50;
        --code-bg: #161b22;
        --code-border: #30363d;
        --shadow: none;
    }
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body {
    font: 16px/1.6 -apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue",
          Arial, sans-serif;
    color: var(--fg);
    background: var(--bg);
}
header.site {
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    position: sticky; top: 0; z-index: 10;
}
header.site .bar {
    max-width: 1100px; margin: 0 auto; padding: 0.9rem 1.5rem;
    display: flex; align-items: center; gap: 1.5rem;
}
header.site .brand {
    font-weight: 700; font-size: 1.1rem; text-decoration: none; color: var(--fg);
}
header.site nav { display: flex; gap: 1.25rem; flex-wrap: wrap; }
header.site nav a {
    color: var(--fg-muted); text-decoration: none; font-size: 0.95rem;
}
header.site nav a:hover, header.site nav a.active {
    color: var(--accent);
}
main {
    max-width: 860px; margin: 0 auto; padding: 2rem 1.5rem 4rem;
}
main.wide { max-width: 1100px; }
h1 { font-size: 2.2rem; line-height: 1.2; margin: 0 0 1rem; }
h2 { border-bottom: 1px solid var(--border); padding-bottom: 0.4rem; margin-top: 2.5rem; }
h3 { margin-top: 1.8rem; }
a { color: var(--accent); }
code {
    font: 0.9em ui-monospace, SFMono-Regular, "SF Mono", Menlo, monospace;
    background: var(--code-bg);
    padding: 0.15em 0.35em;
    border-radius: 4px;
}
pre {
    background: var(--code-bg);
    border: 1px solid var(--code-border);
    border-radius: 6px;
    padding: 1rem;
    overflow-x: auto;
}
pre code { background: none; padding: 0; }
blockquote {
    border-left: 3px solid var(--border);
    margin: 1rem 0; padding: 0.2rem 1rem; color: var(--fg-muted);
}
table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
th, td { border: 1px solid var(--border); padding: 0.5rem 0.75rem; text-align: left; }
th { background: var(--bg-alt); }
img { max-width: 100%; }
hr { border: 0; border-top: 1px solid var(--border); margin: 2rem 0; }

/* Landing-page hero + cards */
.hero { padding: 2.5rem 0 1.5rem; }
.hero h1 { font-size: 2.6rem; }
.hero .lede { font-size: 1.2rem; color: var(--fg-muted); max-width: 640px; }
.cta-row { display: flex; gap: 1rem; margin-top: 1.5rem; flex-wrap: wrap; }
.cta {
    display: inline-block; padding: 0.65rem 1.1rem;
    border-radius: 6px; font-weight: 600; text-decoration: none;
}
.cta.primary { background: var(--accent); color: white; }
.cta.secondary {
    background: var(--bg-alt); color: var(--fg);
    border: 1px solid var(--border);
}
.cards {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 1rem; margin-top: 1.5rem;
}
.card {
    display: block; padding: 1.25rem; border: 1px solid var(--border);
    border-radius: 8px; background: var(--bg); text-decoration: none; color: var(--fg);
    box-shadow: var(--shadow); transition: border-color 0.15s;
}
.card:hover { border-color: var(--accent); }
.card h3 { margin: 0 0 0.4rem; font-size: 1.1rem; }
.card p { margin: 0; color: var(--fg-muted); font-size: 0.95rem; }
.section-label {
    text-transform: uppercase; letter-spacing: 0.05em;
    color: var(--fg-muted); font-size: 0.8rem; font-weight: 600;
    margin: 2.5rem 0 0.5rem;
}
footer.site {
    border-top: 1px solid var(--border); color: var(--fg-muted);
    padding: 1.5rem; text-align: center; font-size: 0.85rem;
}
"""


# ── Rendering helpers ──────────────────────────────────────────────
def _asset_prefix(slug: str) -> str:
    """Relative path back to `assets/` from a page at <slug>/index.html."""
    # Landing page lives at `_site/index.html`; everything else at `_site/<slug>/index.html`.
    return "./" if slug == "" else "../"


def _nav_html(active_slug: str) -> str:
    # Every non-landing page is at `_site/<slug>/index.html`, so the relative
    # prefix back to the site root is "../" — or "./" for the landing page.
    root = "./" if active_slug == "" else "../"
    items: list[str] = []
    for page in PAGES:
        if page.hide_from_nav:
            continue
        cls = " class='active'" if page.slug == active_slug else ""
        items.append(f"<a href='{root}{page.slug}/'{cls}>{page.nav_label}</a>")
    for label, href in EXTERNAL_NAV:
        if href.startswith("."):
            # Relative nav (e.g. "./demo/") is authored from the landing's
            # perspective; rewrite it so links work from sub-pages too.
            resolved = root + href.lstrip("./").rstrip("/") + "/"
        else:
            resolved = href
        items.append(f"<a href='{resolved}'>{label}</a>")
    return (
        f"<header class='site'><div class='bar'>"
        f"<a class='brand' href='{root}'>Forge</a>"
        f"<nav>{''.join(items)}</nav>"
        f"</div></header>"
    )


def _page_shell(
    *, slug: str, title: str, body_html: str, wide: bool = False
) -> str:
    assets = _asset_prefix(slug)
    wide_cls = " wide" if wide else ""
    return (
        "<!DOCTYPE html><html lang='en'><head><meta charset='utf-8'>"
        "<meta name='viewport' content='width=device-width, initial-scale=1'>"
        f"<title>{title} — Forge</title>"
        f"<link rel='stylesheet' href='{assets}assets/docs.css'>"
        "</head><body>"
        f"{_nav_html(slug)}"
        f"<main class='content{wide_cls}'>{body_html}</main>"
        "<footer class='site'>Built from "
        "<a href='https://github.com/grahambrooks/forge'>grahambrooks/forge</a>"
        " on every push to <code>main</code>.</footer>"
        "</body></html>"
    )


def _render_markdown(text: str) -> str:
    md = markdown.Markdown(
        extensions=["fenced_code", "tables", "toc", "sane_lists"],
        extension_configs={
            "toc": {"permalink": False},
        },
    )
    return md.convert(text)


def _write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


# ── Landing page ───────────────────────────────────────────────────
LANDING_INTRO = """\
Forge is a unified software modeling DSL and toolchain. Describe structure
(C4 architecture), processes (CI/CD, branching), deployment topology, and
data flow in one <code>.forge</code> file. Render diagrams, generate
documentation sites, lint architecture, and analyze existing codebases
into inferred models.
"""


def _landing_body() -> str:
    parts: list[str] = []
    parts.append("<section class='hero'>")
    parts.append("<h1>Forge</h1>")
    parts.append(f"<p class='lede'>{LANDING_INTRO}</p>")
    parts.append("<div class='cta-row'>")
    parts.append("<a class='cta primary' href='./demo/'>Explore the demo model →</a>")
    parts.append("<a class='cta secondary' href='./eval/'>See evaluation results</a>")
    parts.append("</div>")
    parts.append("</section>")

    parts.append("<div class='section-label'>Live artifacts</div>")
    parts.append("<div class='cards'>")
    parts.append(
        "<a class='card' href='./demo/'>"
        "<h3>Reference model</h3>"
        "<p>The <code>payments.forge</code> example rendered as a full "
        "documentation site — system context, containers, pipelines, "
        "deployment, data model, and decision records.</p>"
        "</a>"
    )
    parts.append(
        "<a class='card' href='./eval/'>"
        "<h3>Analyzer evaluation</h3>"
        "<p>Results from running <code>forge analyze</code> against real "
        "open-source projects: Flask, Express, Gin, Axum, Sinatra. "
        "Synthesized models with inline SVG previews and drill-down sites.</p>"
        "</a>"
    )
    parts.append("</div>")

    parts.append("<div class='section-label'>Documentation</div>")
    parts.append("<div class='cards'>")
    for p in PAGES:
        if p.hide_from_nav or not p.source.exists():
            continue
        summary = _first_paragraph(p.source)
        parts.append(
            f"<a class='card' href='./{p.slug}/'>"
            f"<h3>{p.title}</h3><p>{summary}</p></a>"
        )
    parts.append("</div>")

    return "".join(parts)


def _first_paragraph(md_path: Path) -> str:
    """Grab the first non-heading paragraph of a markdown file for previews."""
    import html as _html

    lines = md_path.read_text().splitlines()
    buf: list[str] = []
    for line in lines:
        stripped = line.strip()
        if not stripped:
            if buf:
                break
            continue
        if stripped.startswith("#"):
            if buf:
                break
            continue
        if stripped.startswith("```") or stripped.startswith("|"):
            if buf:
                break
            continue
        buf.append(stripped)
    paragraph = " ".join(buf)
    # Keep it short.
    if len(paragraph) > 220:
        paragraph = paragraph[:217].rstrip() + "…"
    return _html.escape(paragraph)


# ── Main ───────────────────────────────────────────────────────────
def main() -> int:
    if len(sys.argv) != 2:
        print("usage: build_docs.py <out_site_root>", file=sys.stderr)
        return 2
    out_root = Path(sys.argv[1]).resolve()
    out_root.mkdir(parents=True, exist_ok=True)

    assets_dir = out_root / "assets"
    assets_dir.mkdir(exist_ok=True)
    (assets_dir / "docs.css").write_text(DOCS_CSS)

    # Render each markdown page into <out>/<slug>/index.html.
    for page in PAGES:
        if not page.source.exists():
            print(f"warning: {page.source} not found; skipping", file=sys.stderr)
            continue
        html_body = _render_markdown(page.source.read_text())
        # Wide layout for DESIGN.md — it's long and has tables.
        wide = page.slug in {"design"}
        shell = _page_shell(
            slug=page.slug, title=page.title, body_html=html_body, wide=wide
        )
        _write(out_root / page.slug / "index.html", shell)

    # Landing page.
    landing = _page_shell(slug="", title="Forge", body_html=_landing_body(), wide=True)
    _write(out_root / "index.html", landing)

    print(f"wrote landing + {len(PAGES)} docs page(s) into {out_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

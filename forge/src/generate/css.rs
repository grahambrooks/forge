#![allow(dead_code)]
pub(super) const CSS: &str = r#"/* Forge documentation site — light/dark theme via CSS custom properties */
:root{
  --fg:#1a1a2e;--fg-muted:#555;--fg-faint:#888;
  --bg:#fafbfc;--bg-surface:#fff;--bg-subtle:#f6f8fa;--bg-input:#f0f2f5;
  --border:#e2e6ea;--border-subtle:rgba(0,0,0,0.06);
  --accent:#1168BD;--accent-light:#dbeafe;
  --nav-bg:#1a1a2e;--nav-fg:#c8cdd3;--nav-hover:rgba(255,255,255,0.06);
  --stat-num:#1168BD;
  --badge-person-bg:#e8f0fe;--badge-person-fg:#08427B;
  --badge-system-bg:#dbeafe;--badge-system-fg:#1168BD;
  --badge-container-bg:#e0f0ff;--badge-container-fg:#438DD5;
  --badge-component-bg:#f0f7ff;--badge-component-fg:#6BA3D6;
  --badge-pipeline-bg:#f3e8ff;--badge-pipeline-fg:#7c3aed;
  --badge-stage-bg:#f5f5f5;--badge-stage-fg:#555;
  --badge-repo-bg:#ecfdf5;--badge-repo-fg:#059669;
  --code-bg:#1a1a2e;--code-fg:#e0e0e0;
  --diff-banner-bg:#f0f7ff;--diff-banner-border:#b8d4f0;
  --blockquote-bg:#f6f8fa;
  color-scheme:light dark;
}
@media(prefers-color-scheme:dark){
  :root{
    --fg:#e0e4ea;--fg-muted:#a0a8b4;--fg-faint:#707880;
    --bg:#0d1117;--bg-surface:#161b22;--bg-subtle:#1c2128;--bg-input:#21262d;
    --border:#30363d;--border-subtle:rgba(255,255,255,0.06);
    --accent:#58a6ff;--accent-light:#1a3a5c;
    --nav-bg:#010409;--nav-fg:#b0b8c4;--nav-hover:rgba(255,255,255,0.08);
    --stat-num:#58a6ff;
    --badge-person-bg:#1a3050;--badge-person-fg:#79b8ff;
    --badge-system-bg:#1a3a5c;--badge-system-fg:#58a6ff;
    --badge-container-bg:#1a3050;--badge-container-fg:#79b8ff;
    --badge-component-bg:#1e2d40;--badge-component-fg:#a5d0f5;
    --badge-pipeline-bg:#2a1a3e;--badge-pipeline-fg:#c49bff;
    --badge-stage-bg:#21262d;--badge-stage-fg:#b0b8c4;
    --badge-repo-bg:#0d2818;--badge-repo-fg:#56d364;
    --code-bg:#010409;--code-fg:#e0e4ea;
    --diff-banner-bg:#1a2332;--diff-banner-border:#1a3a5c;
    --blockquote-bg:#161b22;
  }
}
*,*::before,*::after{box-sizing:border-box}
body{margin:0;font-family:system-ui,-apple-system,'Segoe UI',Helvetica,Arial,sans-serif;color:var(--fg);background:var(--bg);line-height:1.6}
a{color:var(--accent);text-decoration:none}
a:hover{text-decoration:underline}
code{background:var(--bg-input);padding:2px 6px;border-radius:3px;font-size:0.9em}
h1{margin:0 0 0.5em;font-size:1.75rem;font-weight:700}
h2{margin:1.5em 0 0.5em;font-size:1.25rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:0.3em}

/* Layout */
.forge-layout{display:flex;min-height:100vh}
.forge-nav{width:260px;background:var(--nav-bg);color:var(--nav-fg);padding:1rem 0;flex-shrink:0;font-size:0.875rem;overflow-y:auto;position:sticky;top:0;height:100vh}
.forge-nav a{color:var(--nav-fg);display:block;padding:4px 1.25rem}
.forge-nav a:hover{color:#fff;background:var(--nav-hover);text-decoration:none}
.forge-nav__home{font-weight:700;font-size:1rem;color:#fff!important;padding:0.5rem 1.25rem 1rem!important;border-bottom:1px solid rgba(255,255,255,0.08);margin-bottom:0.5rem}
.forge-nav__section{margin:0.5rem 0}
.forge-nav__section>span{display:block;padding:0.5rem 1.25rem 0.2rem;font-size:0.7rem;text-transform:uppercase;letter-spacing:0.05em;color:var(--fg-faint);font-weight:600}
.forge-nav ul{list-style:none;margin:0;padding:0}
.forge-nav li a{padding-left:2rem}
.forge-main{flex:1;padding:2rem 3rem;max-width:1100px}

/* Description */
.forge-desc{font-size:1.05rem;color:var(--fg-muted);margin-bottom:1.5em}

/* Stats row */
.forge-stats{display:flex;gap:1.5rem;margin:1.5em 0;flex-wrap:wrap}
.forge-stat{background:var(--bg-surface);border:1px solid var(--border);border-radius:8px;padding:1rem 1.5rem;text-align:center;min-width:100px}
.forge-stat__num{display:block;font-size:1.75rem;font-weight:700;color:var(--stat-num)}

/* Cards */
.forge-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:1rem;margin:1em 0}
.forge-card{display:block;background:var(--bg-surface);border:1px solid var(--border);border-radius:8px;padding:1.25rem;transition:box-shadow .15s}
.forge-card:hover{box-shadow:0 2px 8px rgba(0,0,0,0.15);text-decoration:none}
.forge-card__title{font-weight:600;font-size:1rem;margin-bottom:0.3em}
.forge-card__sub{font-size:0.85rem;color:var(--fg-faint)}

/* Tables */
.forge-table{width:100%;border-collapse:collapse;margin:0.5em 0 1.5em;font-size:0.9rem}
.forge-table th,.forge-table td{text-align:left;padding:0.5rem 0.75rem;border-bottom:1px solid var(--border)}
.forge-table thead th{background:var(--bg-subtle);font-weight:600;font-size:0.8rem;text-transform:uppercase;letter-spacing:0.03em;color:var(--fg-muted)}
.forge-props th{width:140px;color:var(--fg-muted);font-weight:500;background:var(--bg-subtle)}

/* Badges */
.forge-badge{display:inline-block;font-size:0.7rem;font-weight:600;padding:2px 8px;border-radius:10px;text-transform:uppercase;letter-spacing:0.03em;vertical-align:middle}
.forge-badge--person{background:var(--badge-person-bg);color:var(--badge-person-fg)}
.forge-badge--system{background:var(--badge-system-bg);color:var(--badge-system-fg)}
.forge-badge--container{background:var(--badge-container-bg);color:var(--badge-container-fg)}
.forge-badge--component{background:var(--badge-component-bg);color:var(--badge-component-fg)}
.forge-badge--pipeline{background:var(--badge-pipeline-bg);color:var(--badge-pipeline-fg)}
.forge-badge--stage{background:var(--badge-stage-bg);color:var(--badge-stage-fg)}
.forge-badge--repository{background:var(--badge-repo-bg);color:var(--badge-repo-fg)}

/* Tags */
.forge-tag{display:inline-block;font-size:0.75rem;background:var(--bg-input);color:var(--fg-muted);padding:2px 8px;border-radius:3px;margin:1px}

/* Severity */
.forge-sev--error{color:#f85149;font-weight:600}
.forge-sev--warning{color:#d29922;font-weight:600}
.forge-sev--info{color:var(--accent)}

/* Check summary */
.forge-checks-summary{font-size:0.9rem;color:var(--fg-muted);margin-bottom:0.5em}

/* Diagram */
.forge-diagram-wrap{margin:1em 0;overflow-x:auto}
.forge-diagram-wrap svg{max-width:100%;height:auto}

/* Diff */
.forge-diff-banner{background:var(--diff-banner-bg);border:1px solid var(--diff-banner-border);border-radius:8px;padding:1.25rem 1.5rem;margin:1.5em 0}
.forge-diff-banner h2{margin:0 0 0.5em;border:none;padding:0;font-size:1.2rem}
.forge-diff-desc{margin:0.3em 0 0.8em;color:var(--fg);font-size:1rem}
.forge-diff-stats{display:flex;gap:1.5rem;font-weight:600;font-size:0.9rem}
.forge-diff--added{color:#3fb950}
.forge-diff--modified{color:#d29922}
.forge-diff--removed{color:#f85149}
.forge-diff-rationale{margin-top:1em;padding-top:0.8em;border-top:1px solid var(--border);font-size:0.95rem}
.forge-diff-legend{display:flex;gap:1.5rem;font-size:0.85rem;font-weight:600;margin:0.5em 0 1em;padding:0.5em 0.75em;background:var(--bg-subtle);border-radius:4px;width:fit-content}
.forge-diff-highlight--added{outline:3px solid #3fb950;outline-offset:2px;border-radius:4px}
.forge-diff-highlight--modified{outline:3px solid #d29922;outline-offset:2px;border-radius:4px}

/* Documentation */
.forge-doc{line-height:1.7}
.forge-doc h2{margin:1.8em 0 0.5em;font-size:1.25rem;font-weight:600;border-bottom:1px solid var(--border);padding-bottom:0.3em}
.forge-doc h3{margin:1.4em 0 0.4em;font-size:1.1rem;font-weight:600}
.forge-doc p{margin:0.6em 0}
.forge-doc ul,.forge-doc ol{margin:0.5em 0;padding-left:1.5em}
.forge-doc li{margin:0.3em 0}
.forge-doc pre{background:var(--code-bg);color:var(--code-fg);padding:1rem;border-radius:6px;overflow-x:auto;font-size:0.85rem;line-height:1.5}
.forge-doc code{background:var(--bg-input);padding:2px 6px;border-radius:3px;font-size:0.88em}
.forge-doc pre code{background:none;padding:0;font-size:1em}
.forge-doc blockquote{border-left:3px solid var(--accent);margin:1em 0;padding:0.5em 1em;background:var(--blockquote-bg);color:var(--fg-muted)}
.forge-doc table{border-collapse:collapse;margin:1em 0;width:100%}
.forge-doc th,.forge-doc td{padding:0.5rem 0.75rem;border:1px solid var(--border);text-align:left}
.forge-doc th{background:var(--bg-subtle);font-weight:600}
.forge-doc img{max-width:100%}

/* Animation hint */
.forge-anim-hint{font-size:0.85rem;color:var(--fg-faint);margin-top:0.5em}

/* Responsive */
@media(max-width:768px){
  .forge-layout{flex-direction:column}
  .forge-nav{width:100%;height:auto;position:static;display:flex;flex-wrap:wrap;gap:0.5rem;padding:0.75rem}
  .forge-nav__section{display:none}
  .forge-main{padding:1rem}
}
"#;

// ─── Diff SVG Highlighting ───────────────────────────────────────

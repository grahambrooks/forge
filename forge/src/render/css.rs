//! Default and outline CSS bundled into every rendered SVG.

use super::Colors;

pub(super) fn default_css() -> String {
    // CSS custom properties enable dark mode in both standalone SVGs and embedded in HTML
    format!(
        r#"
    .forge-diagram {{
      font-family: 'Inter', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
      --svg-bg: #ffffff; --svg-fg: #333; --svg-fg-muted: #555; --svg-fg-faint: #888;
      --svg-surface: #fff; --svg-surface-alt: #f8f9fa; --svg-border: #ccc;
      --svg-rel-line: {rel_line}; --svg-pipe-line: {pipe_line};
      --svg-pill-bg: #fff; --svg-pill-opacity: 0.85;
      --svg-stage-bg: {stage_bg}; --svg-stage-stroke: {stage_stroke}; --svg-stage-fg: #333;
      --svg-gate-bg: {gate_bg}; --svg-gate-stroke: {gate_stroke}; --svg-gate-fg: #BF360C;
      --svg-deploy-bg: #f8f9fa; --svg-deploy-stroke: #888;
      --svg-branch-bg: #f0f4ff; --svg-branch-stroke: #5b8def; --svg-branch-fg: #1a3a6b;
      --svg-git0: #4CAF50; --svg-git1: #FF9800; --svg-git2: #2196F3; --svg-git3: #9C27B0;
      --svg-git4: #F44336; --svg-git5: #00BCD4; --svg-git6: #795548; --svg-git7: #607D8B;
    }}
    @media(prefers-color-scheme:dark) {{
      .forge-diagram {{
        --svg-bg: #0d1117; --svg-fg: #e0e4ea; --svg-fg-muted: #a0a8b4; --svg-fg-faint: #707880;
        --svg-surface: #161b22; --svg-surface-alt: #1c2128; --svg-border: #30363d;
        --svg-rel-line: #8b949e; --svg-pipe-line: #8b949e;
        --svg-pill-bg: #161b22; --svg-pill-opacity: 0.95;
        --svg-stage-bg: #1c2128; --svg-stage-stroke: #484f58; --svg-stage-fg: #e0e4ea;
        --svg-gate-bg: #2a1a00; --svg-gate-stroke: #d29922; --svg-gate-fg: #d29922;
        --svg-deploy-bg: #161b22; --svg-deploy-stroke: #484f58;
        --svg-branch-bg: #111d2e; --svg-branch-stroke: #58a6ff; --svg-branch-fg: #79b8ff;
        --svg-git0: #66BB6A; --svg-git1: #FFB74D; --svg-git2: #42A5F5; --svg-git3: #AB47BC;
        --svg-git4: #EF5350; --svg-git5: #26C6DA; --svg-git6: #8D6E63; --svg-git7: #78909C;
      }}
    }}
    .forge-bg {{ fill: var(--svg-bg); }}
    .forge-title {{ font-size: 20px; font-weight: 600; fill: var(--svg-fg); text-anchor: middle; }}
    .forge-shadow {{ filter: url(#dropShadow); }}
    .forge-element text {{ text-anchor: middle; }}
    .forge-label--name {{ font-size: 14px; font-weight: 600; }}
    .forge-label--desc {{ font-size: 11px; font-weight: 400; opacity: 0.85; }}
    .forge-label--tech {{ font-size: 11px; font-weight: 400; font-style: italic; opacity: 0.7; }}
    .forge-label--kind {{ font-size: 10px; font-weight: 400; opacity: 0.55; }}

    .forge-element--person .forge-person-head {{ fill: {person_bg}; }}
    .forge-element--person .forge-person-body {{ fill: {person_bg}; }}
    .forge-element--person rect {{ fill: {person_bg}; stroke: {person_stroke}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--person .forge-label--name {{ fill: #fff; }}
    .forge-element--person .forge-label--desc {{ fill: #b0c4de; }}
    .forge-element--person .forge-label--kind {{ fill: #7a9cc6; }}

    .forge-element--system rect {{ fill: {system_bg}; stroke: {system_stroke}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--system .forge-label--name {{ fill: #fff; }}
    .forge-element--system .forge-label--desc {{ fill: #c0d8ec; }}
    .forge-element--system .forge-label--tech {{ fill: #a8cce8; }}
    .forge-element--system .forge-label--kind {{ fill: #8bb8de; }}

    .forge-element--container rect {{ fill: {container_bg}; stroke: {container_stroke}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--container .forge-label--name {{ fill: #fff; }}
    .forge-element--container .forge-label--desc {{ fill: #d4e6f5; }}
    .forge-element--container .forge-label--tech {{ fill: #c0d8ec; }}
    .forge-element--container .forge-label--kind {{ fill: #a0c4e0; }}

    .forge-element--component rect {{ fill: {component_bg}; stroke: {component_stroke}; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--component .forge-label--name {{ fill: #fff; }}
    .forge-element--component .forge-label--desc {{ fill: #e0eef8; }}
    .forge-element--component .forge-label--tech {{ fill: #c0d8ec; }}

    .forge-element--database .forge-label--name {{ fill: #fff; }}
    .forge-element--database .forge-label--tech {{ fill: #c0d8ec; }}
    .forge-element--database .forge-label--kind {{ fill: #a0c4e0; }}

    .forge-element--stage rect {{ fill: var(--svg-stage-bg); stroke: var(--svg-stage-stroke); stroke-width: 2; rx: 6; ry: 6; }}
    .forge-element--stage .forge-label--name {{ fill: var(--svg-stage-fg); }}
    .forge-element--stage .forge-label--desc {{ fill: var(--svg-fg-muted); }}
    .forge-element--stage .forge-label--tech {{ fill: var(--svg-fg-muted); }}

    .forge-element--gate polygon {{ fill: var(--svg-gate-bg); stroke: var(--svg-gate-stroke); stroke-width: 2; }}
    .forge-element--gate .forge-label--name {{ fill: var(--svg-gate-fg); font-size: 9px; }}

    .forge-element--deploymentnode rect {{ fill: var(--svg-deploy-bg); stroke: var(--svg-deploy-stroke); stroke-width: 1.5; stroke-dasharray: 6,3; rx: 6; ry: 6; }}
    .forge-element--deploymentnode .forge-label--name {{ fill: var(--svg-fg); font-size: 13px; }}
    .forge-element--deploymentnode .forge-label--tech {{ fill: var(--svg-fg-faint); font-size: 11px; }}

    .forge-element--branch .forge-label--name {{ fill: var(--svg-fg); font-size: 13px; font-weight: 600; font-family: 'SF Mono', 'Fira Code', monospace; }}
    .forge-element--branch .forge-label--tech {{ fill: var(--svg-fg-faint); font-size: 10px; }}
    .forge-gitgraph-line {{ stroke-width: 3; fill: none; }}
    .forge-gitgraph-commit {{ stroke: var(--svg-bg); stroke-width: 2; }}
    .forge-gitgraph-merge {{ stroke: var(--svg-bg); stroke-width: 2; }}
    .forge-gitgraph-branch-path {{ stroke-width: 3; fill: none; }}

    .forge-relationship line {{ stroke: var(--svg-rel-line); stroke-width: 1.5; }}
    .forge-relationship path {{ stroke: var(--svg-rel-line); stroke-width: 1.5; fill: none; }}
    .forge-relationship--arrow {{ fill: var(--svg-rel-line); }}
    .forge-label--rel {{ font-size: 11px; fill: var(--svg-fg-muted); text-anchor: middle; }}
    .forge-label--rel-tech {{ font-size: 10px; fill: var(--svg-fg-faint); font-style: italic; text-anchor: middle; }}

    .forge-connector line {{ stroke: var(--svg-pipe-line); stroke-width: 2.5; stroke-dasharray: 8,4; }}
    .forge-connector--arrow {{ fill: var(--svg-pipe-line); }}

    .forge-legend rect.forge-legend-bg {{ fill: var(--svg-surface); stroke: var(--svg-border); stroke-width: 1; rx: 4; ry: 4; }}
    .forge-legend text {{ font-size: 10px; fill: var(--svg-fg-muted); }}
    .forge-legend .forge-legend-title {{ font-size: 11px; font-weight: 600; fill: var(--svg-fg); }}
    .forge-legend rect.forge-legend-swatch {{ stroke-width: 1; rx: 2; ry: 2; }}

    .forge-pill {{ fill: var(--svg-pill-bg); fill-opacity: var(--svg-pill-opacity); }}

    /* ── Entity table ── */
    .forge-entity-box {{ fill: var(--svg-surface, #fff); stroke: {container_bg}; stroke-width: 1.5; rx: 4; ry: 4; }}
    .forge-entity-header {{ fill: {container_bg}; }}
    .forge-entity-header-text {{ fill: #fff; }}
    .forge-entity-sub {{ fill: #c0d8ec; text-anchor: end; }}
    .forge-entity-sep {{ stroke: var(--svg-border, #e2e6ea); stroke-width: 0.5; }}
    .forge-entity-field {{ fill: var(--svg-fg, #333); font-size: 11px; font-weight: 600; text-anchor: start; }}
    .forge-entity-type {{ fill: var(--svg-fg-faint, #888); font-size: 10px; text-anchor: end; }}
"#,
        person_bg = Colors::PERSON_BG,
        person_stroke = Colors::PERSON_STROKE,
        system_bg = Colors::SYSTEM_BG,
        system_stroke = Colors::SYSTEM_STROKE,
        container_bg = Colors::CONTAINER_BG,
        container_stroke = Colors::CONTAINER_STROKE,
        component_bg = Colors::COMPONENT_BG,
        component_stroke = Colors::COMPONENT_STROKE,
        stage_bg = Colors::STAGE_BG,
        stage_stroke = Colors::STAGE_STROKE,
        gate_bg = Colors::GATE_BG,
        gate_stroke = Colors::GATE_STROKE,
        rel_line = Colors::REL_LINE,
        pipe_line = Colors::PIPE_LINE,
    )
}

pub(super) const OUTLINE_CSS: &str = r#"
    .forge-diagram {
      font-family: 'Inter', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
      --svg-bg: #ffffff; --svg-fg: #333; --svg-fg-muted: #555; --svg-fg-faint: #888;
      --svg-surface: #fff; --svg-border: #ccc;
      --svg-rel-line: #707070; --svg-pipe-line: #9E9E9E;
      --svg-pill-bg: #fff; --svg-pill-opacity: 0.85;
      --svg-stage-stroke: #9E9E9E; --svg-gate-stroke: #E65100; --svg-gate-fg: #BF360C;
      --svg-deploy-stroke: #888; --svg-branch-stroke: #5b8def; --svg-branch-fg: #1a3a6b;
      --svg-git0: #4CAF50; --svg-git1: #FF9800; --svg-git2: #2196F3; --svg-git3: #9C27B0;
      --svg-git4: #F44336; --svg-git5: #00BCD4; --svg-git6: #795548; --svg-git7: #607D8B;
    }
    @media(prefers-color-scheme:dark) {
      .forge-diagram {
        --svg-bg: #0d1117; --svg-fg: #e0e4ea; --svg-fg-muted: #a0a8b4; --svg-fg-faint: #707880;
        --svg-surface: #161b22; --svg-border: #30363d;
        --svg-rel-line: #8b949e; --svg-pipe-line: #8b949e;
        --svg-pill-bg: #161b22; --svg-pill-opacity: 0.95;
        --svg-stage-stroke: #484f58; --svg-gate-stroke: #d29922; --svg-gate-fg: #d29922;
        --svg-deploy-stroke: #484f58; --svg-branch-stroke: #58a6ff; --svg-branch-fg: #79b8ff;
        --svg-git0: #66BB6A; --svg-git1: #FFB74D; --svg-git2: #42A5F5; --svg-git3: #AB47BC;
        --svg-git4: #EF5350; --svg-git5: #26C6DA; --svg-git6: #8D6E63; --svg-git7: #78909C;
      }
    }
    .forge-bg { fill: var(--svg-bg); }
    .forge-title { font-size: 20px; font-weight: 600; fill: var(--svg-fg); text-anchor: middle; }
    .forge-shadow { filter: none; }
    .forge-element text { text-anchor: middle; }
    .forge-label--name { font-size: 14px; font-weight: 600; }
    .forge-label--desc { font-size: 11px; font-weight: 400; opacity: 0.7; }
    .forge-label--tech { font-size: 11px; font-weight: 400; font-style: italic; opacity: 0.6; }
    .forge-label--kind { font-size: 10px; font-weight: 400; opacity: 0.5; }

    .forge-element--person .forge-person-head { fill: none; stroke: #08427B; stroke-width: 2; }
    .forge-element--person .forge-person-body { fill: none; stroke: #08427B; stroke-width: 2; }
    .forge-element--person rect { fill: none; stroke: #08427B; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--person .forge-label--name { fill: #08427B; }
    .forge-element--person .forge-label--desc { fill: var(--svg-fg-muted); }
    .forge-element--person .forge-label--kind { fill: var(--svg-fg-faint); }

    .forge-element--system rect { fill: none; stroke: #1168BD; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--system .forge-label--name { fill: #1168BD; }
    .forge-element--system .forge-label--desc { fill: var(--svg-fg-muted); }
    .forge-element--system .forge-label--tech { fill: var(--svg-fg-faint); }
    .forge-element--system .forge-label--kind { fill: var(--svg-fg-faint); }

    .forge-element--container rect { fill: none; stroke: #438DD5; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--container .forge-label--name { fill: #438DD5; }
    .forge-element--container .forge-label--desc { fill: var(--svg-fg-muted); }
    .forge-element--container .forge-label--tech { fill: var(--svg-fg-faint); }
    .forge-element--container .forge-label--kind { fill: var(--svg-fg-faint); }

    .forge-element--component rect { fill: none; stroke: #6BA3D6; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--component .forge-label--name { fill: #6BA3D6; }
    .forge-element--component .forge-label--desc { fill: var(--svg-fg-muted); }
    .forge-element--component .forge-label--tech { fill: var(--svg-fg-faint); }

    .forge-element--database .forge-label--name { fill: #438DD5; }
    .forge-element--database .forge-label--tech { fill: var(--svg-fg-faint); }
    .forge-element--database .forge-label--kind { fill: var(--svg-fg-faint); }

    .forge-element--stage rect { fill: none; stroke: var(--svg-stage-stroke); stroke-width: 2; rx: 6; ry: 6; }
    .forge-element--stage .forge-label--name { fill: var(--svg-fg); }
    .forge-element--stage .forge-label--desc { fill: var(--svg-fg-muted); }
    .forge-element--stage .forge-label--tech { fill: var(--svg-fg-muted); }

    .forge-element--gate polygon { fill: none; stroke: var(--svg-gate-stroke); stroke-width: 2; }
    .forge-element--gate .forge-label--name { fill: var(--svg-gate-fg); font-size: 9px; }

    .forge-element--deploymentnode rect { fill: none; stroke: var(--svg-deploy-stroke); stroke-width: 1.5; stroke-dasharray: 6,3; rx: 6; ry: 6; }
    .forge-element--deploymentnode .forge-label--name { fill: var(--svg-fg); font-size: 13px; }
    .forge-element--deploymentnode .forge-label--tech { fill: var(--svg-fg-faint); font-size: 11px; }

    .forge-element--branch .forge-label--name { fill: var(--svg-fg); font-size: 13px; font-weight: 600; font-family: 'SF Mono', 'Fira Code', monospace; }
    .forge-element--branch .forge-label--tech { fill: var(--svg-fg-faint); font-size: 10px; }
    .forge-gitgraph-line { stroke-width: 3; fill: none; }
    .forge-gitgraph-commit { stroke: var(--svg-bg); stroke-width: 2; }
    .forge-gitgraph-merge { stroke: var(--svg-bg); stroke-width: 2; }
    .forge-gitgraph-branch-path { stroke-width: 3; fill: none; }

    .forge-relationship line { stroke: var(--svg-rel-line); stroke-width: 1.5; }
    .forge-relationship path { stroke: var(--svg-rel-line); stroke-width: 1.5; fill: none; }
    .forge-relationship--arrow { fill: var(--svg-rel-line); }
    .forge-label--rel { font-size: 11px; fill: var(--svg-fg-muted); text-anchor: middle; }
    .forge-label--rel-tech { font-size: 10px; fill: var(--svg-fg-faint); font-style: italic; text-anchor: middle; }

    .forge-connector line { stroke: var(--svg-pipe-line); stroke-width: 2.5; stroke-dasharray: 8,4; }
    .forge-connector--arrow { fill: var(--svg-pipe-line); }

    .forge-legend rect.forge-legend-bg { fill: var(--svg-surface); stroke: var(--svg-border); stroke-width: 1; rx: 4; ry: 4; }
    .forge-legend text { font-size: 10px; fill: var(--svg-fg-muted); }
    .forge-legend .forge-legend-title { font-size: 11px; font-weight: 600; fill: var(--svg-fg); }
    .forge-legend rect.forge-legend-swatch { stroke-width: 2; rx: 2; ry: 2; }

    .forge-pill { fill: var(--svg-pill-bg); fill-opacity: var(--svg-pill-opacity); }

    /* ── Entity table ── */
    .forge-entity-box { fill: var(--svg-surface, #fff); stroke: #438DD5; stroke-width: 1.5; rx: 4; ry: 4; }
    .forge-entity-header { fill: #438DD5; }
    .forge-entity-header-text { fill: #fff; }
    .forge-entity-sub { fill: #c0d8ec; text-anchor: end; }
    .forge-entity-sep { stroke: var(--svg-border, #e2e6ea); stroke-width: 0.5; }
    .forge-entity-field { fill: var(--svg-fg, #333); font-size: 11px; font-weight: 600; text-anchor: start; }
    .forge-entity-type { fill: var(--svg-fg-faint, #888); font-size: 10px; text-anchor: end; }
"#;

//! Forge SVG renderer — clean SVG with semantic CSS classes.
//!
//! Rendering conventions based on Structurizr / C4 model defaults:
//!   - Person: head+shoulders silhouette above a rounded label box
//!   - Software System: large rounded box, background #1168BD
//!   - Container: rounded box, background #438DD5
//!   - Database: cylinder shape
//!   - Stage: rounded box, Gate: diamond
//!   - Drop shadows on all structure elements (filled mode)
//!   - Legend/key box in bottom-right corner

use crate::layout::{Layout, LayoutEdge, LayoutNode, Rect};
use crate::model::ElementKind;

// ─── Canonical C4 colour palette ───

struct Colors;

impl Colors {
    const PERSON_BG: &str = "#08427B";
    const PERSON_STROKE: &str = "#073B6F";
    const SYSTEM_BG: &str = "#1168BD";
    const SYSTEM_STROKE: &str = "#0E4D8B";
    const CONTAINER_BG: &str = "#438DD5";
    const CONTAINER_STROKE: &str = "#3178B9";
    const COMPONENT_BG: &str = "#85BBF0";
    const COMPONENT_STROKE: &str = "#6BA3D6";
    const DATABASE_BG: &str = "#438DD5";
    const DATABASE_STROKE: &str = "#3178B9";
    const STAGE_BG: &str = "#F5F5F5";
    const STAGE_STROKE: &str = "#9E9E9E";
    const GATE_BG: &str = "#FFF3E0";
    const GATE_STROKE: &str = "#E65100";
    const REL_LINE: &str = "#707070";
    const PIPE_LINE: &str = "#9E9E9E";
}

fn default_css() -> String {
    format!(
        r#"
    .forge-diagram {{
      font-family: 'Open Sans', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
    }}
    .forge-title {{
      font-size: 20px; font-weight: 600; fill: #333; text-anchor: middle;
    }}
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

    .forge-element--stage rect {{ fill: {stage_bg}; stroke: {stage_stroke}; stroke-width: 2; rx: 6; ry: 6; }}
    .forge-element--stage .forge-label--name {{ fill: #333; }}
    .forge-element--stage .forge-label--desc {{ fill: #757575; }}
    .forge-element--stage .forge-label--tech {{ fill: #757575; }}

    .forge-element--gate polygon {{ fill: {gate_bg}; stroke: {gate_stroke}; stroke-width: 2; }}
    .forge-element--gate .forge-label--name {{ fill: #BF360C; font-size: 9px; }}

    /* ── Deployment Node ── */
    .forge-element--deploymentnode rect {{ fill: #f8f9fa; stroke: #888; stroke-width: 1.5; stroke-dasharray: 6,3; rx: 6; ry: 6; }}
    .forge-element--deploymentnode .forge-label--name {{ fill: #333; font-size: 13px; }}
    .forge-element--deploymentnode .forge-label--tech {{ fill: #888; font-size: 11px; }}

    /* ── Branch ── */
    .forge-element--branch rect {{ fill: #f0f4ff; stroke: #5b8def; stroke-width: 1.5; rx: 8; ry: 8; }}
    .forge-element--branch .forge-label--name {{ fill: #1a3a6b; font-size: 14px; font-weight: 700; font-family: 'SF Mono', 'Fira Code', monospace; }}
    .forge-element--branch .forge-label--tech {{ fill: #6b7c93; font-size: 10px; }}
    .forge-branch-line {{ stroke: #5b8def; }}
    .forge-branch-dot {{ fill: #5b8def; }}

    .forge-relationship line {{ stroke: {rel_line}; stroke-width: 1.5; }}
    .forge-relationship path {{ stroke: {rel_line}; stroke-width: 1.5; fill: none; }}
    .forge-relationship--arrow {{ fill: {rel_line}; }}
    .forge-label--rel {{ font-size: 11px; fill: #555; text-anchor: middle; }}
    .forge-label--rel-tech {{ font-size: 10px; fill: #888; font-style: italic; text-anchor: middle; }}

    .forge-connector line {{ stroke: {pipe_line}; stroke-width: 2.5; stroke-dasharray: 8,4; }}
    .forge-connector--arrow {{ fill: {pipe_line}; }}

    .forge-legend rect.forge-legend-bg {{ fill: #fff; stroke: #ccc; stroke-width: 1; rx: 4; ry: 4; }}
    .forge-legend text {{ font-size: 10px; fill: #555; }}
    .forge-legend .forge-legend-title {{ font-size: 11px; font-weight: 600; fill: #333; }}
    .forge-legend rect.forge-legend-swatch {{ stroke-width: 1; rx: 2; ry: 2; }}
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

const OUTLINE_CSS: &str = r#"
    .forge-diagram {
      font-family: 'Open Sans', system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif;
    }
    .forge-title {
      font-size: 20px; font-weight: 600; fill: #333; text-anchor: middle;
    }
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
    .forge-element--person .forge-label--desc { fill: #555; }
    .forge-element--person .forge-label--kind { fill: #888; }

    .forge-element--system rect { fill: none; stroke: #1168BD; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--system .forge-label--name { fill: #1168BD; }
    .forge-element--system .forge-label--desc { fill: #555; }
    .forge-element--system .forge-label--tech { fill: #777; }
    .forge-element--system .forge-label--kind { fill: #888; }

    .forge-element--container rect { fill: none; stroke: #438DD5; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--container .forge-label--name { fill: #438DD5; }
    .forge-element--container .forge-label--desc { fill: #555; }
    .forge-element--container .forge-label--tech { fill: #777; }
    .forge-element--container .forge-label--kind { fill: #888; }

    .forge-element--component rect { fill: none; stroke: #6BA3D6; stroke-width: 2; rx: 8; ry: 8; }
    .forge-element--component .forge-label--name { fill: #6BA3D6; }
    .forge-element--component .forge-label--desc { fill: #555; }
    .forge-element--component .forge-label--tech { fill: #777; }

    .forge-element--database .forge-label--name { fill: #438DD5; }
    .forge-element--database .forge-label--tech { fill: #777; }
    .forge-element--database .forge-label--kind { fill: #888; }

    .forge-element--stage rect { fill: none; stroke: #9E9E9E; stroke-width: 2; rx: 6; ry: 6; }
    .forge-element--stage .forge-label--name { fill: #333; }
    .forge-element--stage .forge-label--desc { fill: #757575; }
    .forge-element--stage .forge-label--tech { fill: #757575; }

    .forge-element--gate polygon { fill: none; stroke: #E65100; stroke-width: 2; }
    .forge-element--gate .forge-label--name { fill: #BF360C; font-size: 9px; }

    /* ── Deployment Node (outline) ── */
    .forge-element--deploymentnode rect { fill: none; stroke: #888; stroke-width: 1.5; stroke-dasharray: 6,3; rx: 6; ry: 6; }
    .forge-element--deploymentnode .forge-label--name { fill: #333; font-size: 13px; }
    .forge-element--deploymentnode .forge-label--tech { fill: #888; font-size: 11px; }

    /* ── Branch (outline) ── */
    .forge-element--branch rect { fill: none; stroke: #5b8def; stroke-width: 1.5; rx: 8; ry: 8; }
    .forge-element--branch .forge-label--name { fill: #1a3a6b; font-size: 14px; font-weight: 700; font-family: 'SF Mono', 'Fira Code', monospace; }
    .forge-element--branch .forge-label--tech { fill: #6b7c93; font-size: 10px; }
    .forge-branch-line { stroke: #5b8def; }
    .forge-branch-dot { fill: #5b8def; }

    .forge-relationship line { stroke: #707070; stroke-width: 1.5; }
    .forge-relationship path { stroke: #707070; stroke-width: 1.5; fill: none; }
    .forge-relationship--arrow { fill: #707070; }
    .forge-label--rel { font-size: 11px; fill: #555; text-anchor: middle; }
    .forge-label--rel-tech { font-size: 10px; fill: #888; font-style: italic; text-anchor: middle; }

    .forge-connector line { stroke: #9E9E9E; stroke-width: 2.5; stroke-dasharray: 8,4; }
    .forge-connector--arrow { fill: #9E9E9E; }

    .forge-legend rect.forge-legend-bg { fill: #fff; stroke: #ccc; stroke-width: 1; rx: 4; ry: 4; }
    .forge-legend text { font-size: 10px; fill: #555; }
    .forge-legend .forge-legend-title { font-size: 11px; font-weight: 600; fill: #333; }
    .forge-legend rect.forge-legend-swatch { stroke-width: 2; rx: 2; ry: 2; }
"#;

pub fn render_svg(layout: &Layout, style: &str) -> String {
    let mut o = Vec::new();

    o.push(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}" class="forge-diagram">"#,
        layout.width, layout.height, layout.width, layout.height
    ));
    o.push(format!(
        r##"  <rect width="{}" height="{}" fill="#ffffff" />"##,
        layout.width, layout.height
    ));
    o.push("  <defs>".into());

    let css = if style == "outline" {
        OUTLINE_CSS.to_string()
    } else {
        default_css()
    };
    o.push(format!("    <style>{css}    </style>"));

    // Drop shadow filter
    o.push(r#"    <filter id="dropShadow" x="-4%" y="-4%" width="110%" height="114%">"#.into());
    o.push(
        r##"      <feDropShadow dx="2" dy="3" stdDeviation="3" flood-color="#000" flood-opacity="0.12"/>"##
            .into(),
    );
    o.push("    </filter>".into());

    // Arrowhead markers
    o.push(format!(
        r#"    <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 1 L 8 5 L 0 9 z" fill="{}"/></marker>"#,
        Colors::REL_LINE
    ));
    o.push(format!(
        r#"    <marker id="arrow-pipe" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 1 L 8 5 L 0 9 z" fill="{}"/></marker>"#,
        Colors::PIPE_LINE
    ));
    o.push("  </defs>".into());

    // Title
    if let Some(ref title) = layout.title {
        o.push(format!(
            r#"  <text x="{:.0}" y="30" class="forge-title">{}</text>"#,
            layout.width / 2.0,
            esc(title)
        ));
    }

    // Edges first (under nodes)
    o.push(r#"  <g class="forge-relationships">"#.into());
    for e in &layout.edges {
        render_edge(&mut o, e, &layout.nodes);
    }
    o.push("  </g>".into());

    // Nodes
    o.push(r#"  <g class="forge-elements">"#.into());
    for n in &layout.nodes {
        render_node(&mut o, n, style);
    }
    o.push("  </g>".into());

    // Legend
    render_legend(&mut o, layout, style);

    o.push("</svg>".into());
    let mut result = o.join("\n");
    result.push('\n');
    result
}

// ─── Node Rendering ──────────────────────────────────────────────

fn render_node(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let css = css_kind(n.kind);
    let tag_cls = if n.tags.contains(&"database".to_string()) {
        " forge-element--database"
    } else {
        ""
    };

    o.push(format!(
        r#"    <g class="forge-element forge-element--{css}{tag_cls}" data-id="{}">"#,
        esc(&n.id)
    ));

    if n.tags.contains(&"data-entity".to_string()) {
        render_entity_table(o, n);
    } else if n.kind == ElementKind::Branch {
        render_branch(o, n);
    } else if n.kind == ElementKind::DeploymentNode {
        render_deployment_node(o, n);
    } else if n.kind == ElementKind::Gate {
        render_gate(o, n);
    } else if n.kind == ElementKind::Person {
        render_person(o, n, style);
    } else if n.tags.contains(&"database".to_string()) {
        render_cylinder(o, n, style);
    } else if n.kind == ElementKind::Stage {
        render_stage(o, n);
    } else {
        render_box(o, n);
    }

    o.push("    </g>".into());
}

fn render_person(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;

    let head_r = 20.0;
    let head_cy = r.y + head_r + 2.0;
    let shoulder_y = head_cy + head_r + 2.0;
    let shoulder_w = 36.0;
    let box_y = shoulder_y + 18.0;
    let box_h = r.y + r.h - box_y;

    if style == "outline" {
        let stroke = Colors::PERSON_STROKE;
        let bx = r.x;
        let bw = r.w;
        let rad = 8.0;
        let sl = cx - shoulder_w;
        let sr = cx + shoulder_w;
        let bot = box_y + box_h;

        o.push(format!(
            r#"      <path d="M {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} L {:.0} {:.0} L {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} L {:.0} {:.0} L {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Z" fill="none" stroke="{}" stroke-width="2" />"#,
            bx + rad, bot,
            bx, bot, bx, bot - rad,
            bx, box_y,
            sl, box_y,
            sl, shoulder_y, cx, shoulder_y,
            sr, shoulder_y, sr, box_y,
            bx + bw, box_y,
            bx + bw, bot - rad,
            bx + bw, bot, bx + bw - rad, bot,
            stroke
        ));
        o.push(format!(
            r#"      <circle cx="{:.0}" cy="{:.0}" r="{}" fill="none" stroke="{}" stroke-width="2" />"#,
            cx, head_cy, head_r, stroke
        ));
    } else {
        o.push(format!(
            r#"      <circle cx="{:.0}" cy="{:.0}" r="{}" class="forge-person-head" />"#,
            cx, head_cy, head_r
        ));
        let sl = cx - shoulder_w;
        let sr = cx + shoulder_w;
        o.push(format!(
            r#"      <path d="M {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Q {:.0} {:.0} {:.0} {:.0} Z" class="forge-person-body" />"#,
            sl, box_y, sl, shoulder_y, cx, shoulder_y, sr, shoulder_y, sr, box_y
        ));
        o.push(format!(
            r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
            r.x, box_y, r.w, box_h
        ));
    }

    // Text
    let ty = box_y + 20.0;
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name">{}</text>"#,
        cx,
        ty,
        esc(&n.label)
    ));
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--kind">[Person]</text>"#,
        cx,
        ty + 16.0
    ));
    if let Some(ref desc) = n.description {
        render_wrapped_text(o, cx, ty + 32.0, r.w - 20.0, desc, "forge-label--desc");
    }
}

fn render_box(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let max_text_w = r.w - 24.0;

    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
        r.x, r.y, r.w, r.h
    ));

    // Compute all text lines with wrapping
    struct TextLine {
        cls: &'static str,
        text: String,
    }
    let mut lines: Vec<TextLine> = Vec::new();
    lines.push(TextLine {
        cls: "name",
        text: n.label.clone(),
    });
    if let Some(kl) = kind_label(n.kind) {
        lines.push(TextLine {
            cls: "kind",
            text: format!("[{}]", kl),
        });
    }
    if let Some(ref desc) = n.description {
        // Wrap description to fit box width
        let wrapped = wrap_text_render(desc, max_text_w, 6.0);
        for line in wrapped.into_iter().take(3) {
            lines.push(TextLine {
                cls: "desc",
                text: line,
            });
        }
    }
    if let Some(ref sub) = n.sublabel {
        lines.push(TextLine {
            cls: "tech",
            text: sub.clone(),
        });
    }

    let total_h = lines.len() as f64 * 16.0;
    let mut ty = r.y + (r.h - total_h) / 2.0 + 14.0;

    for line in &lines {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}">{}</text>"#,
            cx,
            ty,
            line.cls,
            esc(&line.text)
        ));
        ty += 16.0;
    }
}

/// Word-wrap text for rendering. Returns lines that fit within max_w.
fn wrap_text_render(text: &str, max_w: f64, char_w: f64) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut line: Vec<&str> = Vec::new();
    let mut est_w = 0.0;
    for w in &words {
        let w_est = w.len() as f64 * char_w;
        if est_w + w_est > max_w && !line.is_empty() {
            lines.push(line.join(" "));
            line = vec![w];
            est_w = w_est;
        } else {
            line.push(w);
            est_w += w_est + char_w * 0.6;
        }
    }
    if !line.is_empty() {
        lines.push(line.join(" "));
    }
    lines
}

fn render_stage(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;

    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" class="forge-shadow" />"#,
        r.x, r.y, r.w, r.h
    ));

    let mut lines: Vec<(&str, String)> = vec![("name", n.label.clone())];
    if let Some(ref sub) = n.sublabel {
        lines.push(("tech", format!("[{}]", sub)));
    }

    let total_h = lines.len() as f64 * 16.0;
    let mut ty = r.y + (r.h - total_h) / 2.0 + 14.0;

    for (cls, text) in &lines {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--{}">{}</text>"#,
            cx,
            ty,
            cls,
            esc(text)
        ));
        ty += 16.0;
    }
}

fn render_entity_table(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let header_h = 32.0;
    let row_h = 20.0;
    let pad = 10.0;

    // Outer box
    o.push(format!(
        r##"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" fill="#fff" stroke="#438DD5" stroke-width="1.5" rx="4" ry="4" />"##,
        r.x, r.y, r.w, r.h
    ));

    // Header background
    o.push(format!(
        r##"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" fill="#438DD5" rx="4" ry="0" />"##,
        r.x, r.y, r.w, header_h
    ));
    // Header corners fix (cover bottom radius)
    o.push(format!(
        r##"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="4" fill="#438DD5" />"##,
        r.x,
        r.y + header_h - 4.0,
        r.w
    ));

    // Entity name in header
    o.push(format!(
        r##"      <text x="{:.0}" y="{:.0}" class="forge-label--name" fill="#fff">{}</text>"##,
        r.x + r.w / 2.0,
        r.y + 21.0,
        esc(&n.label)
    ));

    // Owner subtitle
    if let Some(ref sub) = n.sublabel {
        o.push(format!(
            r##"      <text x="{:.0}" y="{:.0}" font-size="9" fill="#c0d8ec" text-anchor="end">{}</text>"##,
            r.x + r.w - pad,
            r.y + 21.0,
            esc(sub)
        ));
    }

    // Field rows from description (newline-separated)
    if let Some(ref desc) = n.description {
        let fields: Vec<&str> = desc.split('\n').collect();
        let mut fy = r.y + header_h + pad + 12.0;
        for field in fields {
            // Split "name: type (constraints)"
            let (fname, frest) = field.split_once(':').unwrap_or((field, ""));
            let frest = frest.trim();

            // Row separator line
            if fy > r.y + header_h + pad + 12.0 {
                o.push(format!(
                    r##"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="#e2e6ea" stroke-width="0.5" />"##,
                    r.x + pad,
                    fy - row_h + 4.0,
                    r.x + r.w - pad,
                    fy - row_h + 4.0
                ));
            }

            // Field name (left-aligned, bold)
            o.push(format!(
                r##"      <text x="{:.0}" y="{:.0}" font-size="11" font-weight="600" fill="#333" text-anchor="start">{}</text>"##,
                r.x + pad,
                fy,
                esc(fname.trim())
            ));

            // Field type + constraints (right-aligned)
            o.push(format!(
                r##"      <text x="{:.0}" y="{:.0}" font-size="10" fill="#888" text-anchor="end">{}</text>"##,
                r.x + r.w - pad,
                fy,
                esc(frest)
            ));

            fy += row_h;
        }
    }
}

fn render_branch(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let is_trunk = n.tags.contains(&"trunk".to_string());

    // Branch lane rectangle
    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" />"#,
        r.x, r.y, r.w, r.h
    ));

    // Branch line (horizontal through the middle)
    let line_y = r.y + r.h / 2.0;
    let line_x1 = r.x + 120.0;
    let line_x2 = r.x + r.w - 20.0;
    let sw = if is_trunk { "3" } else { "2" };
    let dash = if is_trunk {
        ""
    } else {
        r#" stroke-dasharray="8,4""#
    };
    o.push(format!(
        r#"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" class="forge-branch-line" stroke-width="{}"{} />"#,
        line_x1, line_y, line_x2, line_y, sw, dash
    ));

    // Dot at branch start
    o.push(format!(
        r#"      <circle cx="{:.0}" cy="{:.0}" r="5" class="forge-branch-dot" />"#,
        line_x1, line_y
    ));

    // Branch name (left side)
    let label_y = r.y + r.h / 2.0 + 5.0;
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" text-anchor="start">{}</text>"#,
        r.x + 14.0, label_y, esc(&n.label)
    ));

    // Sublabel (protection or flow description, right side)
    if let Some(ref sub) = n.sublabel {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--tech" text-anchor="end">{}</text>"#,
            r.x + r.w - 14.0, label_y, esc(sub)
        ));
    }
}

fn render_deployment_node(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    // Dashed border rect for the deployment node boundary
    o.push(format!(
        r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" />"#,
        r.x, r.y, r.w, r.h
    ));
    // Header label (top-left)
    let lx = r.x + 10.0;
    let ly = r.y + 18.0;
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" text-anchor="start">{}</text>"#,
        lx, ly, esc(&n.label)
    ));
    if let Some(ref sub) = n.sublabel {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--tech" text-anchor="start">{}</text>"#,
            lx, ly + 14.0, esc(sub)
        ));
    }
}

fn render_gate(o: &mut Vec<String>, n: &LayoutNode) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let cy = r.y + r.h / 2.0;
    let pts = format!(
        "{:.0},{:.0} {:.0},{:.0} {:.0},{:.0} {:.0},{:.0}",
        cx,
        r.y,
        r.x + r.w,
        cy,
        cx,
        r.y + r.h,
        r.x,
        cy
    );
    o.push(format!(r#"      <polygon points="{}" />"#, pts));

    let words: Vec<&str> = n.label.split('-').collect();
    if n.label.len() <= 14 {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy + 4.0, esc(&n.label)
        ));
    } else {
        let mid = words.len() / 2;
        let l1 = words[..mid].join("-");
        let l2 = words[mid..].join("-");
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy - 4.0, esc(&l1)
        ));
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name" dominant-baseline="central">{}</text>"#,
            cx, cy + 10.0, esc(&l2)
        ));
    }
}

fn render_cylinder(o: &mut Vec<String>, n: &LayoutNode, style: &str) {
    let r = &n.rect;
    let cx = r.x + r.w / 2.0;
    let ry = 12.0;
    let rx_half = r.w / 2.0;
    let stroke = Colors::DATABASE_STROKE;

    if style == "outline" {
        let top_y = r.y + ry;
        let bot_y = r.y + r.h - ry;
        let lx = r.x;
        let rx_right = r.x + r.w;

        o.push(format!(
            r#"      <path d="M {:.0} {:.0} A {:.0} {} 0 0 1 {:.0} {:.0} L {:.0} {:.0} A {:.0} {} 0 0 1 {:.0} {:.0} Z" fill="none" stroke="{}" stroke-width="2" />"#,
            lx, top_y, rx_half, ry, rx_right, top_y,
            rx_right, bot_y, rx_half, ry, lx, bot_y,
            stroke
        ));
        o.push(format!(
            r#"      <path d="M {:.0} {:.0} A {:.0} {} 0 0 0 {:.0} {:.0}" fill="none" stroke="{}" stroke-width="2" />"#,
            lx, top_y, rx_half, ry, rx_right, top_y, stroke
        ));
    } else {
        let bg = Colors::DATABASE_BG;
        let sw = "1.5";
        o.push(format!(
            r#"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" fill="{}" stroke="{}" stroke-width="{}" rx="0" ry="0" />"#,
            r.x, r.y + ry, r.w, r.h - 2.0 * ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <ellipse cx="{:.0}" cy="{:.0}" rx="{:.0}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            cx, r.y + ry, rx_half, ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <ellipse cx="{:.0}" cy="{:.0}" rx="{:.0}" ry="{}" fill="{}" stroke="{}" stroke-width="{}" />"#,
            cx, r.y + r.h - ry, rx_half, ry, bg, stroke, sw
        ));
        o.push(format!(
            r#"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="{}" />"#,
            r.x, r.y + ry, r.x, r.y + r.h - ry, stroke, sw
        ));
        o.push(format!(
            r#"      <line x1="{:.0}" y1="{:.0}" x2="{:.0}" y2="{:.0}" stroke="{}" stroke-width="{}" />"#,
            r.x + r.w, r.y + ry, r.x + r.w, r.y + r.h - ry, stroke, sw
        ));
    }

    let ty = r.y + r.h / 2.0 - 4.0;
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--name">{}</text>"#,
        cx,
        ty,
        esc(&n.label)
    ));
    if let Some(ref sub) = n.sublabel {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="forge-label--tech">{}</text>"#,
            cx,
            ty + 16.0,
            esc(sub)
        ));
    }
    o.push(format!(
        r#"      <text x="{:.0}" y="{:.0}" class="forge-label--kind">[Database]</text>"#,
        cx,
        ty + 32.0
    ));
}

// ─── Edge Rendering ──────────────────────────────────────────────

fn render_edge(o: &mut Vec<String>, edge: &LayoutEdge, nodes: &[LayoutNode]) {
    let frm = nodes.iter().find(|n| n.id == edge.frm);
    let to = nodes.iter().find(|n| n.id == edge.to);
    let (frm, to) = match (frm, to) {
        (Some(f), Some(t)) => (f, t),
        _ => return,
    };

    let fx = frm.rect.x + frm.rect.w / 2.0;
    let fy = frm.rect.y + frm.rect.h / 2.0;
    let tx = to.rect.x + to.rect.w / 2.0;
    let ty = to.rect.y + to.rect.h / 2.0;

    let (ex1, ey1) = edge_point(&frm.rect, tx, ty);
    let (ex2, ey2) = edge_point(&to.rect, fx, fy);

    let is_pipe = frm.kind == ElementKind::Stage || to.kind == ElementKind::Stage;
    let cls = if is_pipe {
        "forge-connector"
    } else {
        "forge-relationship"
    };
    let marker = if is_pipe {
        "url(#arrow-pipe)"
    } else {
        "url(#arrow)"
    };

    o.push(format!(
        r#"    <g class="{}" data-from="{}" data-to="{}">"#,
        cls,
        esc(&edge.frm),
        esc(&edge.to)
    ));
    o.push(format!(
        r#"      <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" marker-end="{}" />"#,
        ex1, ey1, ex2, ey2, marker
    ));

    if !edge.label.is_empty() {
        // Place label at the middle third of the edge to avoid node overlap
        let mx = (ex1 + ex2) / 2.0;
        let my = (ey1 + ey2) / 2.0;

        // For near-vertical edges, offset the label horizontally to avoid overlap
        let dx = (ex2 - ex1).abs();
        let dy = (ey2 - ey1).abs();
        let (mx, my) = if dy > dx * 2.0 {
            // Near-vertical: offset label to the right
            (mx + 40.0, my)
        } else {
            (mx, my)
        };

        let label_text = &edge.label;
        let tech_text = edge.technology.as_ref().map(|t| format!("[{}]", t));
        let label_y = if tech_text.is_some() {
            my - 14.0
        } else {
            my - 8.0
        };

        let pill_w = f64::max(
            label_text.len() as f64,
            tech_text.as_ref().map_or(0.0, |t| t.len() as f64),
        ) * 6.5
            + 16.0;
        let pill_h = if tech_text.is_some() { 34.0 } else { 20.0 };
        let pill_x = mx - pill_w / 2.0;
        let pill_y = label_y - 12.0;
        o.push(format!(
            r##"      <rect x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" rx="4" ry="4" fill="#fff" fill-opacity="0.85" />"##,
            pill_x, pill_y, pill_w, pill_h
        ));
        o.push(format!(
            r#"      <text x="{:.1}" y="{:.1}" class="forge-label--rel">{}</text>"#,
            mx,
            label_y,
            esc(label_text)
        ));
        if let Some(ref tt) = tech_text {
            o.push(format!(
                r#"      <text x="{:.1}" y="{:.1}" class="forge-label--rel-tech">{}</text>"#,
                mx,
                label_y + 14.0,
                esc(tt)
            ));
        }
    }

    o.push("    </g>".into());
}

// ─── Legend ───────────────────────────────────────────────────────

fn render_legend(o: &mut Vec<String>, layout: &Layout, style: &str) {
    use std::collections::HashMap;

    let mut kinds_seen: HashMap<(ElementKind, Option<&str>), &LayoutNode> = HashMap::new();
    for n in &layout.nodes {
        let tag = if n.tags.contains(&"database".to_string()) {
            Some("database")
        } else {
            None
        };
        let key = (n.kind, tag);
        kinds_seen.entry(key).or_insert(n);
    }

    if kinds_seen.is_empty() {
        return;
    }

    let mut entries: Vec<(String, &str)> = Vec::new();
    for (kind, tag) in kinds_seen.keys() {
        let (label, color) = if *tag == Some("database") {
            ("Database".to_string(), Colors::DATABASE_BG)
        } else {
            let label = kind_label(*kind).unwrap_or_else(|| format!("{:?}", kind));
            let color = match kind {
                ElementKind::Person => Colors::PERSON_BG,
                ElementKind::System => Colors::SYSTEM_BG,
                ElementKind::Container => Colors::CONTAINER_BG,
                ElementKind::Component => Colors::COMPONENT_BG,
                ElementKind::Stage => Colors::STAGE_BG,
                ElementKind::Gate => Colors::GATE_BG,
                ElementKind::DeploymentNode => "#f8f9fa",
                _ => "#ccc",
            };
            (label, color)
        };
        entries.push((label, color));
    }

    let row_h = 20.0;
    let pad = 10.0;
    let legend_w = 160.0;
    let legend_h = pad * 2.0 + entries.len() as f64 * row_h + 16.0;

    let lx = layout.width - legend_w - 20.0;
    let ly = layout.height - legend_h - 10.0;

    o.push(r#"  <g class="forge-legend">"#.into());
    o.push(format!(
        r#"    <rect class="forge-legend-bg" x="{:.0}" y="{:.0}" width="{}" height="{:.0}" />"#,
        lx, ly, legend_w, legend_h
    ));
    o.push(format!(
        r#"    <text x="{:.0}" y="{:.0}" class="forge-legend-title">Legend</text>"#,
        lx + pad,
        ly + pad + 10.0
    ));

    let mut ey = ly + pad + 26.0;
    for (label, color) in &entries {
        let (swatch_fill, swatch_stroke) = if style == "outline" {
            ("none", *color)
        } else {
            (*color, *color)
        };
        o.push(format!(
            r#"    <rect class="forge-legend-swatch" x="{:.0}" y="{:.0}" width="14" height="14" fill="{}" stroke="{}" />"#,
            lx + pad,
            ey - 8.0,
            swatch_fill,
            swatch_stroke
        ));
        o.push(format!(
            r#"    <text x="{:.0}" y="{:.0}">{}</text>"#,
            lx + pad + 22.0,
            ey + 3.0,
            esc(label)
        ));
        ey += row_h;
    }
    o.push("  </g>".into());
}

// ─── Utilities ───────────────────────────────────────────────────

fn edge_point(r: &Rect, tx: f64, ty: f64) -> (f64, f64) {
    let cx = r.x + r.w / 2.0;
    let cy = r.y + r.h / 2.0;
    let dx = tx - cx;
    let dy = ty - cy;
    if dx.abs() < 0.01 && dy.abs() < 0.01 {
        return (cx, cy);
    }
    let hw = r.w / 2.0;
    let hh = r.h / 2.0;
    let sx = if dx.abs() > 0.01 { hw / dx.abs() } else { 1e9 };
    let sy = if dy.abs() > 0.01 { hh / dy.abs() } else { 1e9 };
    let s = f64::min(sx, sy);
    (cx + dx * s, cy + dy * s)
}

fn render_wrapped_text(
    o: &mut Vec<String>,
    cx: f64,
    mut y: f64,
    max_w: f64,
    text: &str,
    cls: &str,
) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut line: Vec<&str> = Vec::new();
    let mut est_w = 0.0;
    for w in &words {
        let w_est = w.len() as f64 * 6.5;
        if est_w + w_est > max_w && !line.is_empty() {
            lines.push(line.join(" "));
            line = vec![w];
            est_w = w_est;
        } else {
            line.push(w);
            est_w += w_est + 4.0;
        }
    }
    if !line.is_empty() {
        lines.push(line.join(" "));
    }
    for l in lines.iter().take(3) {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="{}">{}</text>"#,
            cx,
            y,
            cls,
            esc(l)
        ));
        y += 14.0;
    }
}

fn kind_label(kind: ElementKind) -> Option<String> {
    match kind {
        ElementKind::Person => Some("Person".into()),
        ElementKind::System => Some("Software System".into()),
        ElementKind::Container => Some("Container".into()),
        ElementKind::Component => Some("Component".into()),
        ElementKind::Pipeline => Some("Pipeline".into()),
        ElementKind::Repository => Some("Repository".into()),
        ElementKind::DeploymentNode => Some("Deployment Node".into()),
        ElementKind::Branch => Some("Branch".into()),
        ElementKind::Stage | ElementKind::Gate => None,
        _ => None,
    }
}

fn css_kind(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Stage => "stage",
        ElementKind::Gate => "gate",
        ElementKind::Pipeline => "pipeline",
        ElementKind::Repository => "repository",
        ElementKind::DeploymentNode => "deploymentnode",
        ElementKind::Branch => "branch",
        _ => "element",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout, parser};

    fn render_view(view_key: &str, style: &str) -> String {
        let text = include_str!("../examples/payments.forge");
        let model = parser::parse(text).unwrap();
        let view = model.views.iter().find(|v| v.key == view_key).unwrap();
        let lo = layout::compute_layout(&model, view);
        render_svg(&lo, style)
    }

    #[test]
    fn svg_is_valid_xml_structure() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.starts_with("<svg "));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    }

    #[test]
    fn svg_contains_title() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("Payment Platform"));
        assert!(svg.contains("class=\"forge-title\""));
    }

    #[test]
    fn svg_contains_element_groups() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("class=\"forge-elements\""));
        assert!(svg.contains("class=\"forge-relationships\""));
    }

    #[test]
    fn svg_contains_person_element() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("forge-element--person"));
        assert!(svg.contains("Customer"));
        assert!(svg.contains("[Person]"));
    }

    #[test]
    fn svg_contains_system_element() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("forge-element--system"));
        assert!(svg.contains("Payment Service"));
    }

    #[test]
    fn svg_filled_has_drop_shadow() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("id=\"dropShadow\""));
        assert!(svg.contains("filter: url(#dropShadow)"));
    }

    #[test]
    fn svg_outline_disables_shadow() {
        let svg = render_view("SystemContext", "outline");
        assert!(svg.contains("filter: none"));
    }

    #[test]
    fn svg_outline_uses_no_fill() {
        let svg = render_view("SystemContext", "outline");
        assert!(svg.contains("fill: none"));
    }

    #[test]
    fn svg_contains_relationship_labels() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("forge-label--rel"));
    }

    #[test]
    fn svg_containers_has_database_cylinder() {
        let svg = render_view("Containers", "filled");
        assert!(svg.contains("forge-element--database"));
        assert!(svg.contains("ellipse")); // cylinder cap
        assert!(svg.contains("[Database]"));
    }

    #[test]
    fn svg_containers_outline_cylinder() {
        let svg = render_view("Containers", "outline");
        assert!(svg.contains("forge-element--database"));
        // outline cylinder uses path arcs, not ellipse
        assert!(svg.contains("<path d=\"M"));
    }

    #[test]
    fn svg_containers_has_technology_labels() {
        let svg = render_view("Containers", "filled");
        assert!(svg.contains("[Rust / Actix]"));
        assert!(svg.contains("[PostgreSQL 16]"));
        assert!(svg.contains("[Redis]"));
    }

    #[test]
    fn svg_pipeline_has_stages() {
        let svg = render_view("Pipeline", "filled");
        assert!(svg.contains("forge-element--stage"));
        assert!(svg.contains("Build &amp; Test"));
        assert!(svg.contains("Security Scan"));
        assert!(svg.contains("Deploy Staging"));
        assert!(svg.contains("Deploy Production"));
    }

    #[test]
    fn svg_pipeline_has_gates() {
        let svg = render_view("Pipeline", "filled");
        assert!(svg.contains("forge-element--gate"));
        assert!(svg.contains("<polygon")); // diamond shape
    }

    #[test]
    fn svg_pipeline_has_connectors() {
        let svg = render_view("Pipeline", "filled");
        assert!(svg.contains("forge-connector"));
        assert!(svg.contains("arrow-pipe"));
    }

    #[test]
    fn svg_contains_legend() {
        let svg = render_view("SystemContext", "filled");
        assert!(svg.contains("forge-legend"));
        assert!(svg.contains("Legend"));
    }

    #[test]
    fn svg_legend_outline_swatches() {
        let svg = render_view("SystemContext", "outline");
        assert!(svg.contains("forge-legend-swatch"));
        assert!(svg.contains(r#"fill="none""#));
    }

    #[test]
    fn svg_has_white_background() {
        let svg = render_view("Containers", "filled");
        assert!(svg.contains(r##"fill="#ffffff""##));
    }

    #[test]
    fn svg_has_arrowhead_markers() {
        let svg = render_view("Containers", "filled");
        assert!(svg.contains("id=\"arrow\""));
        assert!(svg.contains("marker-end="));
    }

    #[test]
    fn esc_html_entities() {
        assert_eq!(esc("a<b>c&d\"e"), "a&lt;b&gt;c&amp;d&quot;e");
    }

    #[test]
    fn all_views_render_both_styles() {
        for key in &["SystemContext", "Containers", "Pipeline", "Deployment"] {
            for style in &["filled", "outline"] {
                let svg = render_view(key, style);
                assert!(svg.contains("<svg "), "{key}/{style} missing svg tag");
                assert!(svg.contains("</svg>"), "{key}/{style} missing closing svg");
            }
        }
    }

    #[test]
    fn svg_deployment_has_nested_nodes() {
        let svg = render_view("Deployment", "filled");
        assert!(svg.contains("forge-element--deploymentnode"));
        assert!(svg.contains("EKS Cluster"));
        assert!(svg.contains("stroke-dasharray")); // dashed borders in CSS
    }

    #[test]
    fn svg_deployment_has_container_instances() {
        let svg = render_view("Deployment", "filled");
        // Container instances should appear inside deployment nodes
        assert!(svg.contains("Payment API") || svg.contains("Ledger DB"));
    }
}

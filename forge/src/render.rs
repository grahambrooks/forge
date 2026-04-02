/// Forge SVG renderer — produces clean SVG with semantic CSS classes.

use crate::layout::{Layout, LayoutEdge, LayoutNode, Rect};
use crate::model::ElementKind;

/// Default embedded stylesheet.
const DEFAULT_STYLES: &str = r#"
    .forge-diagram { font-family: system-ui, -apple-system, 'Segoe UI', sans-serif; }
    .forge-title { font-size: 18px; font-weight: 600; fill: #333; }

    /* Element base */
    .forge-element rect, .forge-element polygon { stroke-width: 1.5; rx: 6; ry: 6; }
    .forge-element text { text-anchor: middle; }
    .forge-label { font-size: 14px; font-weight: 500; }
    .forge-label--sub { font-size: 11px; font-weight: 400; fill-opacity: 0.8; }

    /* Person */
    .forge-element--person rect { fill: #08427B; stroke: #073B6F; }
    .forge-element--person .forge-label { fill: #ffffff; }
    .forge-element--person .forge-label--sub { fill: #b0c4de; }
    .forge-element--person-icon { fill: #ffffff; }

    /* System */
    .forge-element--system rect { fill: #1168BD; stroke: #0f5ca8; }
    .forge-element--system .forge-label { fill: #ffffff; }
    .forge-element--system .forge-label--sub { fill: #a8cce8; }

    /* Container */
    .forge-element--container rect { fill: #438DD5; stroke: #3a7ebf; }
    .forge-element--container .forge-label { fill: #ffffff; }
    .forge-element--container .forge-label--sub { fill: #c8ddf0; }

    /* Container with database tag */
    .forge-element--database rect { fill: #1168BD; stroke: #0f5ca8; rx: 0; ry: 0; }
    .forge-element--database .forge-label { fill: #ffffff; }
    .forge-element--database .forge-label--sub { fill: #a8cce8; }

    /* Stage */
    .forge-element--stage rect { fill: #F5F5F5; stroke: #BDBDBD; stroke-width: 2; rx: 4; ry: 4; }
    .forge-element--stage .forge-label { fill: #333333; }
    .forge-element--stage .forge-label--sub { fill: #757575; }

    /* Gate */
    .forge-element--gate polygon { fill: #FFF3E0; stroke: #E65100; stroke-width: 2; }
    .forge-element--gate .forge-label { fill: #E65100; font-size: 10px; }

    /* Relationships */
    .forge-relationship line, .forge-relationship path { stroke: #707070; stroke-width: 1.5; fill: none; }
    .forge-relationship--arrow { fill: #707070; }
    .forge-label--relationship { font-size: 11px; fill: #555; }
    .forge-label--technology { font-size: 10px; fill: #888; font-style: italic; }

    /* Pipeline connector */
    .forge-connector line { stroke: #BDBDBD; stroke-width: 2; stroke-dasharray: 6,3; }
    .forge-connector--arrow { fill: #BDBDBD; }
"#;

pub fn render_svg(layout: &Layout) -> String {
    let mut svg = String::with_capacity(4096);

    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}" class="forge-diagram">
"#,
        layout.width, layout.height, layout.width, layout.height
    ));

    // Embedded styles
    svg.push_str("  <defs>\n    <style>\n");
    svg.push_str(DEFAULT_STYLES);
    svg.push_str("\n    </style>\n");

    // Arrowhead marker
    svg.push_str(r#"    <marker id="arrow" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="forge-relationship--arrow"/>
    </marker>
    <marker id="arrow-pipeline" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="forge-connector--arrow"/>
    </marker>
"#);
    svg.push_str("  </defs>\n\n");

    // Title
    if let Some(ref title) = layout.title {
        svg.push_str(&format!(
            r#"  <text x="{}" y="24" class="forge-title">{}</text>
"#,
            layout.width / 2.0,
            escape_xml(title)
        ));
    }

    // Render edges first (under nodes)
    svg.push_str("  <g class=\"forge-relationships\">\n");
    for edge in &layout.edges {
        render_edge(&mut svg, edge, &layout.nodes);
    }
    svg.push_str("  </g>\n\n");

    // Render nodes
    svg.push_str("  <g class=\"forge-elements\">\n");
    for node in &layout.nodes {
        render_node(&mut svg, node);
    }
    svg.push_str("  </g>\n");

    svg.push_str("</svg>\n");
    svg
}

fn render_node(svg: &mut String, node: &LayoutNode) {
    let r = &node.rect;
    let css_kind = css_class_for_kind(&node.kind);
    let tag_class = if node.tags.contains(&"database".to_string()) {
        " forge-element--database"
    } else {
        ""
    };

    svg.push_str(&format!(
        "    <g class=\"forge-element forge-element--{}{}\" data-id=\"{}\">\n",
        css_kind,
        tag_class,
        escape_xml(&node.id)
    ));

    match node.kind {
        ElementKind::Gate => {
            // Diamond shape
            let cx = r.x + r.width / 2.0;
            let cy = r.y + r.height / 2.0;
            let hw = r.width / 2.0;
            let hh = r.height / 2.0;
            svg.push_str(&format!(
                "      <polygon points=\"{},{} {},{} {},{} {},{}\" />\n",
                cx, r.y,           // top
                r.x + r.width, cy, // right
                cx, r.y + r.height, // bottom
                r.x, cy            // left
            ));
            svg.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" class=\"forge-label\" dominant-baseline=\"central\">{}</text>\n",
                cx, cy,
                escape_xml(&truncate(&node.label, 12))
            ));
        }
        ElementKind::Person => {
            // Box with person icon
            svg.push_str(&format!(
                "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n",
                r.x, r.y, r.width, r.height
            ));
            // Simple person icon (circle + body)
            let icon_cx = r.x + r.width / 2.0;
            let icon_y = r.y + 18.0;
            svg.push_str(&format!(
                "      <circle cx=\"{}\" cy=\"{}\" r=\"8\" class=\"forge-element--person-icon\" />\n",
                icon_cx, icon_y
            ));
            svg.push_str(&format!(
                "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"white\" stroke-width=\"2\" />\n",
                icon_cx, icon_y + 8.0, icon_cx, icon_y + 22.0
            ));
            // Label
            svg.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" class=\"forge-label\">{}</text>\n",
                r.x + r.width / 2.0, r.y + r.height - 28.0,
                escape_xml(&node.label)
            ));
            if let Some(ref sub) = node.sublabel {
                svg.push_str(&format!(
                    "      <text x=\"{}\" y=\"{}\" class=\"forge-label--sub\">{}</text>\n",
                    r.x + r.width / 2.0, r.y + r.height - 12.0,
                    escape_xml(sub)
                ));
            }
        }
        _ => {
            // Rounded rectangle (or cylinder for database — simplified as rect with different style)
            if node.tags.contains(&"database".to_string()) {
                render_cylinder(svg, r);
            } else {
                svg.push_str(&format!(
                    "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n",
                    r.x, r.y, r.width, r.height
                ));
            }
            // Label
            let label_y = if node.sublabel.is_some() {
                r.y + r.height / 2.0 - 6.0
            } else {
                r.y + r.height / 2.0 + 5.0
            };
            svg.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" class=\"forge-label\" dominant-baseline=\"central\">{}</text>\n",
                r.x + r.width / 2.0, label_y,
                escape_xml(&node.label)
            ));
            if let Some(ref sub) = node.sublabel {
                svg.push_str(&format!(
                    "      <text x=\"{}\" y=\"{}\" class=\"forge-label--sub\" dominant-baseline=\"central\">{}</text>\n",
                    r.x + r.width / 2.0, label_y + 18.0,
                    escape_xml(sub)
                ));
            }
        }
    }

    svg.push_str("    </g>\n");
}

fn render_cylinder(svg: &mut String, r: &Rect) {
    let ry = 10.0; // ellipse radius for cylinder top/bottom
    // Body
    svg.push_str(&format!(
        "      <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"0\" ry=\"0\" />\n",
        r.x, r.y + ry, r.width, r.height - ry * 2.0
    ));
    // Top ellipse
    svg.push_str(&format!(
        "      <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#1168BD\" stroke=\"#0f5ca8\" stroke-width=\"1.5\" />\n",
        r.x + r.width / 2.0, r.y + ry, r.width / 2.0, ry
    ));
    // Bottom ellipse (just the lower half visible)
    svg.push_str(&format!(
        "      <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#1168BD\" stroke=\"#0f5ca8\" stroke-width=\"1.5\" />\n",
        r.x + r.width / 2.0, r.y + r.height - ry, r.width / 2.0, ry
    ));
    // Side lines
    svg.push_str(&format!(
        "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f5ca8\" stroke-width=\"1.5\" />\n",
        r.x, r.y + ry, r.x, r.y + r.height - ry
    ));
    svg.push_str(&format!(
        "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f5ca8\" stroke-width=\"1.5\" />\n",
        r.x + r.width, r.y + ry, r.x + r.width, r.y + r.height - ry
    ));
}

fn render_edge(svg: &mut String, edge: &LayoutEdge, nodes: &[LayoutNode]) {
    let from_node = nodes.iter().find(|n| n.id == edge.from);
    let to_node = nodes.iter().find(|n| n.id == edge.to);

    if let (Some(from), Some(to)) = (from_node, to_node) {
        let (x1, y1) = center_of(&from.rect);
        let (x2, y2) = center_of(&to.rect);

        // Compute edge endpoints at rect boundaries
        let (ex1, ey1) = edge_point(&from.rect, x2, y2);
        let (ex2, ey2) = edge_point(&to.rect, x1, y1);

        let is_pipeline = from.kind == ElementKind::Stage || to.kind == ElementKind::Stage;
        let class = if is_pipeline { "forge-connector" } else { "forge-relationship" };
        let marker = if is_pipeline { "url(#arrow-pipeline)" } else { "url(#arrow)" };

        svg.push_str(&format!(
            "    <g class=\"{}\" data-from=\"{}\" data-to=\"{}\">\n",
            class,
            escape_xml(&edge.from),
            escape_xml(&edge.to)
        ));
        svg.push_str(&format!(
            "      <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" marker-end=\"{}\" />\n",
            ex1, ey1, ex2, ey2, marker
        ));

        // Edge label
        if !edge.label.is_empty() {
            let mx = (ex1 + ex2) / 2.0;
            let my = (ey1 + ey2) / 2.0 - 8.0;
            svg.push_str(&format!(
                "      <text x=\"{}\" y=\"{}\" class=\"forge-label--relationship\" text-anchor=\"middle\">{}</text>\n",
                mx, my,
                escape_xml(&edge.label)
            ));
            if let Some(ref tech) = edge.technology {
                svg.push_str(&format!(
                    "      <text x=\"{}\" y=\"{}\" class=\"forge-label--technology\" text-anchor=\"middle\">[{}]</text>\n",
                    mx, my + 14.0,
                    escape_xml(tech)
                ));
            }
        }

        svg.push_str("    </g>\n");
    }
}

fn center_of(r: &Rect) -> (f64, f64) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Find the point on the boundary of a rectangle closest to a target point.
fn edge_point(r: &Rect, tx: f64, ty: f64) -> (f64, f64) {
    let (cx, cy) = center_of(r);
    let dx = tx - cx;
    let dy = ty - cy;

    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return (cx, cy);
    }

    let hw = r.width / 2.0;
    let hh = r.height / 2.0;

    // Scale factor to reach rectangle boundary
    let sx = if dx.abs() > 0.001 { hw / dx.abs() } else { f64::MAX };
    let sy = if dy.abs() > 0.001 { hh / dy.abs() } else { f64::MAX };
    let s = sx.min(sy);

    (cx + dx * s, cy + dy * s)
}

fn css_class_for_kind(kind: &ElementKind) -> &str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Stage => "stage",
        ElementKind::Gate => "gate",
        ElementKind::Pipeline => "pipeline",
        ElementKind::Repository => "repository",
        _ => "element",
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

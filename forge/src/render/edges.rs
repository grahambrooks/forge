//! Relationship edges and the legend in the bottom-right corner.

use super::branching::render_gitgraph_edge;
use super::util::{esc, kind_label};
use super::{Colors, TM};
use crate::layout::{Layout, LayoutEdge, LayoutNode, Rect};
use crate::model::ElementKind;
use crate::text::*;

pub(super) fn render_edge(o: &mut Vec<String>, edge: &LayoutEdge, nodes: &[LayoutNode]) {
    let frm = nodes.iter().find(|n| n.id == edge.frm);
    let to = nodes.iter().find(|n| n.id == edge.to);
    let (frm, to) = match (frm, to) {
        (Some(f), Some(t)) => (f, t),
        _ => return,
    };

    // Gitgraph-style curved branch/merge paths
    if frm.kind == ElementKind::Branch && to.kind == ElementKind::Branch {
        render_gitgraph_edge(o, edge, frm, to);
        return;
    }

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
            TM.measure(label_text, &FONT_REL),
            tech_text
                .as_ref()
                .map_or(0.0, |t| TM.measure(t, &FONT_REL_TECH)),
        ) + 16.0;
        let pill_h = if tech_text.is_some() { 34.0 } else { 20.0 };
        let pill_x = mx - pill_w / 2.0;
        let pill_y = label_y - 12.0;
        o.push(format!(
            r#"      <rect class="forge-pill" x="{:.0}" y="{:.0}" width="{:.0}" height="{:.0}" rx="4" ry="4" />"#,
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

    // Dynamic-view step badge: a filled circle with the step number near
    // the arrow's midpoint, offset to the side of the label so the two
    // don't collide.
    if let Some(step) = edge.order {
        let mx = (ex1 + ex2) / 2.0;
        let my = (ey1 + ey2) / 2.0;
        let dx = ex2 - ex1;
        let dy = ey2 - ey1;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        // Offset the badge 14px perpendicular to the edge so it sits
        // beside the arrow rather than on top of it.
        let nx = -dy / len;
        let ny = dx / len;
        let bx = mx + nx * 14.0;
        let by = my + ny * 14.0;
        let text_y = by + 4.0;
        o.push(format!(
            r##"      <circle cx="{bx:.1}" cy="{by:.1}" r="11" fill="#1f2937" stroke="#f9fafb" stroke-width="2" class="forge-step-badge" />"##
        ));
        o.push(format!(
            r##"      <text x="{bx:.1}" y="{text_y:.1}" class="forge-step-label" text-anchor="middle" font-size="12" font-weight="700" fill="#f9fafb">{step}</text>"##
        ));
    }

    o.push("    </g>".into());
}

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

pub(super) fn render_legend(o: &mut Vec<String>, layout: &Layout, style: &str) {
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

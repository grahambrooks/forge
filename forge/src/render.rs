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
//!
//! The renderer is split across sub-modules by concern. This file holds the
//! shared palette, the text-measurer cache, and the public entry points
//! ([`render_svg`] and [`render_view`]).

use crate::layout::Layout;
use crate::model::{Model, View, ViewKind};
use crate::text::*;
use std::sync::LazyLock;

mod branching;
mod composite;
mod css;
mod edges;
mod entity;
mod shapes;
mod util;

#[cfg(test)]
mod tests;

use util::esc;

static TM: LazyLock<TextMeasurer> = LazyLock::new(TextMeasurer::new);

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

pub fn render_svg(layout: &Layout, style: &str) -> String {
    let mut o = Vec::new();

    o.push(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}" class="forge-diagram">"#,
        layout.width, layout.height, layout.width, layout.height
    ));
    o.push(format!(
        r#"  <rect class="forge-bg" width="{}" height="{}" />"#,
        layout.width, layout.height
    ));
    o.push("  <defs>".into());

    let css_text = if style == "outline" {
        css::OUTLINE_CSS.to_string()
    } else {
        css::default_css()
    };
    o.push(format!("    <style>{css_text}    </style>"));

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

    // Edge lines first (under nodes) — the line+arrowhead get clipped
    // visually by the node fills, which is the look we want.
    o.push(r#"  <g class="forge-relationships">"#.into());
    for e in &layout.edges {
        edges::render_edge(&mut o, e, &layout.nodes);
    }
    o.push("  </g>".into());

    // Nodes
    o.push(r#"  <g class="forge-elements">"#.into());
    for n in &layout.nodes {
        shapes::render_node(&mut o, n, style);
    }
    o.push("  </g>".into());

    // Edge labels and step badges last, so the pill backgrounds sit on
    // top of any node they pass over rather than being overpainted.
    o.push(r#"  <g class="forge-relationship-labels">"#.into());
    for e in &layout.edges {
        edges::render_edge_label(&mut o, e, &layout.nodes);
    }
    o.push("  </g>".into());

    // Legend
    edges::render_legend(&mut o, layout, style);

    o.push("</svg>".into());
    let mut result = o.join("\n");
    result.push('\n');
    result
}

/// Render a view to SVG. For ordinary views this is a thin wrapper over
/// `compute_layout` + `render_svg`. For composite views it dispatches to
/// each referenced cell's normal pipeline and assembles the child SVGs
/// into a row-major grid wrapped in an outer `<svg>`.
pub fn render_view(model: &Model, view: &View, style: &str) -> String {
    if view.kind == ViewKind::Composite {
        return composite::render_composite(model, view, style);
    }
    let lo = crate::layout::compute_layout(model, view);
    render_svg(&lo, style)
}

//! Composite view renderer: assembles multiple child SVGs into a grid.

use super::util::esc;
use crate::model::{Model, View, ViewKind};

pub(super) fn render_composite(model: &Model, view: &View, style: &str) -> String {
    let Some(comp) = view.composite.as_ref() else {
        return String::new();
    };

    let cols = comp.cols.max(1);
    let (cell_w, cell_h) = (comp.cell_size.0 as f64, comp.cell_size.1 as f64);
    let gap = 20.0_f64;
    let pad = 20.0_f64;
    let title_h = 50.0_f64;

    // Compute rows from the number of cells + cols so we never overflow
    // if the user provides fewer cells than rows*cols suggest.
    let filled_rows = comp.cells.len().div_ceil(cols as usize).max(1);
    let rows = (comp.rows as usize).max(filled_rows);

    let canvas_w = pad * 2.0 + cols as f64 * cell_w + (cols.saturating_sub(1)) as f64 * gap;
    let canvas_h =
        pad * 2.0 + title_h + rows as f64 * cell_h + (rows.saturating_sub(1) as f64) * gap;

    let title_text = view.title.as_deref().unwrap_or(&view.key);

    let mut child_svgs: Vec<String> = Vec::new();
    for (idx, cell_key) in comp.cells.iter().enumerate() {
        let Some(cell_view) = model.views.iter().find(|v| v.key == *cell_key) else {
            continue;
        };
        // Prevent infinite recursion: nested composites render as an empty
        // placeholder with a hint label.
        if cell_view.kind == ViewKind::Composite {
            continue;
        }
        let lo = crate::layout::compute_layout(model, cell_view);
        let child = super::render_svg(&lo, style);
        let col = idx as u32 % cols;
        let row = idx as u32 / cols;
        let x = pad + col as f64 * (cell_w + gap);
        let y = pad + title_h + row as f64 * (cell_h + gap);

        let (inner_w, inner_h) = extract_svg_viewbox(&child);
        let inner = strip_outer_svg(&child);
        child_svgs.push(format!(
            r#"  <svg x="{x:.0}" y="{y:.0}" width="{cell_w:.0}" height="{cell_h:.0}" viewBox="0 0 {inner_w:.0} {inner_h:.0}" preserveAspectRatio="xMidYMid meet" class="forge-composite-cell" data-view="{key}">{inner}</svg>"#,
            key = esc(cell_key),
        ));
        // Draw a thin frame around the cell so the grid reads clearly.
        child_svgs.push(format!(
            r##"  <rect x="{x:.0}" y="{y:.0}" width="{cell_w:.0}" height="{cell_h:.0}" fill="none" stroke="#d1d5db" stroke-width="1" rx="6" ry="6" />"##
        ));
        // Small caption at the bottom-right of the cell.
        child_svgs.push(format!(
            r##"  <text x="{tx:.0}" y="{ty:.0}" class="forge-composite-caption" text-anchor="end" font-size="11" fill="#6b7280">{label}</text>"##,
            tx = x + cell_w - 8.0,
            ty = y + cell_h - 8.0,
            label = esc(cell_key),
        ));
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canvas_w:.0} {canvas_h:.0}" width="{canvas_w:.0}" height="{canvas_h:.0}" class="forge-diagram forge-composite">
<defs><style>text{{font-family:system-ui,sans-serif}}.forge-title{{font-size:20px;font-weight:700;fill:#111827}}</style></defs>
<rect width="100%" height="100%" fill="#ffffff" />
<text x="{tx:.0}" y="{ty:.0}" class="forge-title">{title}</text>
{cells}
</svg>"##,
        tx = pad,
        ty = pad + 28.0,
        title = esc(title_text),
        cells = child_svgs.join("\n")
    )
}

fn extract_svg_viewbox(svg: &str) -> (f64, f64) {
    // Best-effort parse of the viewBox="0 0 W H" attribute from the first
    // <svg> tag. Fall back to 1000x600 on any parse failure.
    if let Some(start) = svg.find("viewBox=\"") {
        let rest = &svg[start + "viewBox=\"".len()..];
        if let Some(end) = rest.find('"') {
            let nums: Vec<f64> = rest[..end]
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            if nums.len() >= 4 {
                return (nums[2], nums[3]);
            }
        }
    }
    (1000.0, 600.0)
}

/// Remove the outer `<svg ...>...</svg>` wrapper from a rendered SVG
/// string, returning the inner markup. Assumes the SVG has exactly one
/// top-level `<svg>` element, which `render_svg` guarantees.
fn strip_outer_svg(svg: &str) -> String {
    let Some(open_end) = svg.find('>') else {
        return svg.to_string();
    };
    let after_open = &svg[open_end + 1..];
    let close_start = after_open.rfind("</svg>").unwrap_or(after_open.len());
    after_open[..close_start].to_string()
}

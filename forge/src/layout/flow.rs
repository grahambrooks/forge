use super::container::layout_container;
use super::*;

pub(super) fn layout_dynamic(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    let mut lo = layout_container(model, view, tm);

    // Fall back to the model name + "— Flow" if the view didn't set a title
    // so the page header reads usefully.
    if lo.title.as_deref() == Some(&format!("{} — Containers", model.name)) {
        lo.title = Some(format!("{} — Dynamic Flow", model.name));
    }
    lo
}

// ─── Composite View ──────────────────────────────────────────────

/// Composite views aren't a single layout — they dispatch to each
/// referenced view and the renderer assembles the child SVGs into a
/// grid. This function returns a placeholder layout carrying just the
/// title and canvas dimensions; `render::render_svg` detects the
/// composite kind and re-invokes the layout/render pipeline per cell.
pub(super) fn layout_composite(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let cells = view.composite.as_ref();
    let (cols, rows, cell_w, cell_h) = cells
        .map(|c| (c.cols, c.rows, c.cell_size.0 as f64, c.cell_size.1 as f64))
        .unwrap_or((1, 1, 600.0, 400.0));

    let gap = 20.0;
    let canvas_w = PAD * 2.0 + cols as f64 * cell_w + (cols.saturating_sub(1)) as f64 * gap;
    let canvas_h =
        PAD * 2.0 + TITLE_H + rows as f64 * cell_h + (rows.saturating_sub(1)) as f64 * gap;

    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Composite", model.name));
    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}

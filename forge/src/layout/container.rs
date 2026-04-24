use super::*;
use std::collections::HashSet;

// ─── Container View ──────────────────────────────────────────────

pub(super) fn layout_container(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    let scope_id = view.scope.as_deref().unwrap_or("");
    let system = model.elements.get(scope_id);

    let containers: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Container)
        .collect();

    let container_ids: HashSet<String> = containers.iter().map(|c| c.id.clone()).collect();
    let rels = model.relationships_involving(&container_ids);

    let mut ext_ids: Vec<String> = Vec::new();
    for r in &rels {
        for eid in [&r.frm, &r.to] {
            if !container_ids.contains(eid) && eid.as_str() != scope_id && !ext_ids.contains(eid) {
                ext_ids.push(eid.clone());
            }
        }
    }

    let externals: Vec<&Element> = ext_ids
        .iter()
        .filter_map(|eid| model.elements.get(eid))
        .collect();

    let mut nodes = Vec::new();

    // Externals on top row (centered)
    let ext_total_w: f64 = externals.iter().map(|e| dims_for(e, tm).0).sum::<f64>()
        + H_GAP * (externals.len().saturating_sub(1) as f64);
    let cols: usize = 3;
    let grid_w = cols as f64 * NODE_W + (cols as f64 - 1.0) * H_GAP;
    let canvas_w = f64::max(ext_total_w, grid_w) + PAD * 2.0;
    let mut ext_x = (canvas_w - ext_total_w) / 2.0;

    for ext in &externals {
        let (ew, _) = dims_for(ext, tm);
        nodes.push(make_node(ext, ext_x, TITLE_H, tm));
        ext_x += ew + H_GAP;
    }

    // Containers in a grid
    let y_start = TITLE_H
        + if !externals.is_empty() {
            PERSON_H + V_GAP * 1.3
        } else {
            0.0
        };
    let grid_start_x = (canvas_w - grid_w) / 2.0;

    for (i, cont) in containers.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = grid_start_x + col as f64 * (NODE_W + H_GAP);
        let y = y_start + row as f64 * (NODE_H + V_GAP);
        nodes.push(make_node(cont, x, y, tm));
    }

    let all_ids: HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let view_rels = model.relationships_between(&all_ids);
    let edges: Vec<LayoutEdge> = view_rels
        .iter()
        .map(|r| LayoutEdge {
            frm: r.frm.clone(),
            to: r.to.clone(),
            label: r.label.clone(),
            technology: r.technology.clone(),
            order: r.order,
        })
        .collect();

    let max_x = nodes
        .iter()
        .map(|n| n.rect.x + n.rect.w)
        .fold(400.0_f64, f64::max)
        + PAD;
    let max_y = nodes
        .iter()
        .map(|n| n.rect.y + n.rect.h)
        .fold(200.0_f64, f64::max)
        + PAD
        + 40.0;
    let title = view.title.clone().unwrap_or_else(|| {
        system
            .map(|s| format!("{} — Containers", s.name))
            .unwrap_or_else(|| "Containers".into())
    });
    Layout {
        width: f64::max(max_x, canvas_w),
        height: max_y,
        title: Some(title),
        nodes,
        edges,
    }
}

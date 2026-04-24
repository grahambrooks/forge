use super::*;
use std::collections::HashSet;

pub(super) fn layout_system_context(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    let scope_id = view.scope.as_deref().unwrap_or("");
    let system = model.elements.get(scope_id);

    let child_ids: HashSet<String> = model
        .elements
        .iter()
        .filter(|(_, e)| e.parent.as_deref() == Some(scope_id))
        .map(|(eid, _)| eid.clone())
        .collect();

    let mut scope_set: HashSet<String> = child_ids;
    scope_set.insert(scope_id.to_string());

    let rels = model.relationships_involving(&scope_set);
    let mut actor_ids: Vec<String> = Vec::new();
    for r in &rels {
        let other = if scope_set.contains(&r.frm) {
            &r.to
        } else {
            &r.frm
        };
        if !scope_set.contains(other) && !actor_ids.contains(other) {
            actor_ids.push(other.clone());
        }
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Actors on the left column
    let mut y_cursor = TITLE_H;
    for aid in &actor_ids {
        if let Some(actor) = model.elements.get(aid) {
            nodes.push(make_node(actor, PAD, y_cursor, tm));
            y_cursor += dims_for(actor, tm).1 + V_GAP;
        }
    }
    let actor_col_h = y_cursor - V_GAP + PAD;

    // System on the right
    let sys_x = PAD + PERSON_W + H_GAP * 2.5;
    let sys_y = TITLE_H + f64::max(0.0, (actor_col_h - TITLE_H - SYSTEM_H) / 2.0);
    if let Some(sys) = system {
        nodes.push(LayoutNode {
            id: sys.id.clone(),
            label: sys.name.clone(),
            sublabel: None,
            kind: sys.kind,
            tags: sys.tags.clone(),
            rect: Rect {
                x: sys_x,
                y: sys_y,
                w: SYSTEM_W,
                h: SYSTEM_H,
            },
            description: sys.description.clone(),
            depth: 0,
            children_ids: Vec::new(),
            data_classes: sys.data_classes.clone(),
        });
    }

    // Collapse edges to system level
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for r in &rels {
        let frm = if scope_set.contains(&r.frm) {
            scope_id.to_string()
        } else {
            r.frm.clone()
        };
        let to = if scope_set.contains(&r.to) {
            scope_id.to_string()
        } else {
            r.to.clone()
        };
        let key = (frm.clone(), to.clone());
        if seen.insert(key) {
            edges.push(LayoutEdge {
                frm,
                to,
                label: r.label.clone(),
                technology: r.technology.clone(),
                order: None,
            });
        }
    }

    let w = sys_x + SYSTEM_W + PAD;
    let h = f64::max(actor_col_h, sys_y + SYSTEM_H + PAD);
    let title = view.title.clone().unwrap_or_else(|| {
        system
            .map(|s| format!("{} — System Context", s.name))
            .unwrap_or_else(|| "System Context".into())
    });
    Layout {
        width: w,
        height: h,
        title: Some(title),
        nodes,
        edges,
    }
}

/// Forge layout engine — assigns (x, y) positions to elements for SVG rendering.
///
/// Prototype implements two layout strategies:
/// - Layered (Sugiyama-lite): for container and system context views
/// - Pipeline: left-to-right stage flow for CI/CD views

use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    pub sublabel: Option<String>,
    pub kind: ElementKind,
    pub tags: Vec<String>,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
}

#[derive(Debug)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

// ───── Constants ─────

const NODE_W: f64 = 200.0;
const NODE_H: f64 = 100.0;
const STAGE_W: f64 = 180.0;
const STAGE_H: f64 = 80.0;
const GATE_SIZE: f64 = 50.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 60.0;
const PADDING: f64 = 40.0;

/// Compute layout for a container view.
pub fn layout_container_view(model: &Model, view: &View) -> Layout {
    let scope_id = view.scope.as_deref().unwrap_or("");

    // Collect the system element and all external actors
    let system = model.elements.get(scope_id);

    // Gather containers inside the system
    let containers: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Container)
        .collect();

    // Gather people / external systems that interact
    let container_ids: Vec<&str> = containers.iter().map(|c| c.id.as_str()).collect();
    let rels = model.relationships_involving(&container_ids);

    let mut external_ids: Vec<String> = Vec::new();
    for r in &rels {
        if !container_ids.contains(&r.from.as_str()) && !external_ids.contains(&r.from) {
            external_ids.push(r.from.clone());
        }
        if !container_ids.contains(&r.to.as_str()) && !external_ids.contains(&r.to) {
            external_ids.push(r.to.clone());
        }
    }

    let externals: Vec<&Element> = external_ids
        .iter()
        .filter_map(|id| model.elements.get(id))
        .collect();

    // Layout: externals on top row, containers on bottom rows (grid)
    let mut nodes = Vec::new();
    let mut x_offset = PADDING;
    let y_ext = PADDING;

    for ext in &externals {
        nodes.push(LayoutNode {
            id: ext.id.clone(),
            label: ext.name.clone(),
            sublabel: match ext.kind {
                ElementKind::Person => Some("[Person]".into()),
                ElementKind::System => Some("[External System]".into()),
                _ => None,
            },
            kind: ext.kind.clone(),
            tags: ext.tags.clone(),
            rect: Rect { x: x_offset, y: y_ext, width: NODE_W, height: NODE_H },
        });
        x_offset += NODE_W + H_GAP;
    }

    let ext_row_width = x_offset - H_GAP + PADDING;

    // Container grid: 3 columns
    let cols = 3usize;
    let y_start = if externals.is_empty() { PADDING } else { y_ext + NODE_H + V_GAP * 2.0 };
    let mut max_x = ext_row_width;

    for (i, cont) in containers.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = PADDING + col as f64 * (NODE_W + H_GAP);
        let y = y_start + row as f64 * (NODE_H + V_GAP);

        nodes.push(LayoutNode {
            id: cont.id.clone(),
            label: cont.name.clone(),
            sublabel: cont.technology.clone().map(|t| format!("[{}]", t)),
            kind: cont.kind.clone(),
            tags: cont.tags.clone(),
            rect: Rect { x, y, width: NODE_W, height: NODE_H },
        });

        let right = x + NODE_W + PADDING;
        if right > max_x {
            max_x = right;
        }
    }

    let last_row = if containers.is_empty() { 0 } else { (containers.len() - 1) / cols };
    let total_height = y_start + (last_row + 1) as f64 * (NODE_H + V_GAP) + PADDING;

    // Build edges
    let all_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let view_rels = model.relationships_between(&all_ids);
    let edges: Vec<LayoutEdge> = view_rels
        .iter()
        .map(|r| LayoutEdge {
            from: r.from.clone(),
            to: r.to.clone(),
            label: r.label.clone(),
            technology: r.technology.clone(),
        })
        .collect();

    Layout {
        width: max_x,
        height: total_height,
        title: view.title.clone().or_else(|| system.map(|s| format!("{} — Containers", s.name))),
        nodes,
        edges,
    }
}

/// Compute layout for a system context view.
pub fn layout_system_context_view(model: &Model, view: &View) -> Layout {
    let scope_id = view.scope.as_deref().unwrap_or("");
    let system = model.elements.get(scope_id);

    // The central system + all people/systems connected to it
    let rels_from: Vec<&Relationship> = model.relationships.iter()
        .filter(|r| r.from == scope_id || r.to == scope_id
            || r.from.starts_with(&format!("{}.", scope_id))
            || r.to.starts_with(&format!("{}.", scope_id)))
        .collect();

    let mut actor_ids: Vec<String> = Vec::new();
    for r in &rels_from {
        let other = if r.from == scope_id || r.from.starts_with(&format!("{}.", scope_id)) {
            &r.to
        } else {
            &r.from
        };
        if !other.starts_with(&format!("{}.", scope_id)) && other != scope_id && !actor_ids.contains(other) {
            actor_ids.push(other.clone());
        }
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Place actors on the left, system in the center
    let y_start = PADDING;
    let x_actors = PADDING;
    let x_system = PADDING + NODE_W + H_GAP * 2.0;

    for (i, aid) in actor_ids.iter().enumerate() {
        if let Some(actor) = model.elements.get(aid) {
            nodes.push(LayoutNode {
                id: actor.id.clone(),
                label: actor.name.clone(),
                sublabel: match actor.kind {
                    ElementKind::Person => Some("[Person]".into()),
                    _ => Some("[External System]".into()),
                },
                kind: actor.kind.clone(),
                tags: actor.tags.clone(),
                rect: Rect {
                    x: x_actors,
                    y: y_start + i as f64 * (NODE_H + V_GAP),
                    width: NODE_W,
                    height: NODE_H,
                },
            });
        }
    }

    let sys_y = y_start + if actor_ids.is_empty() { 0.0 } else {
        ((actor_ids.len() as f64 - 1.0) / 2.0) * (NODE_H + V_GAP)
    };

    if let Some(sys) = system {
        nodes.push(LayoutNode {
            id: sys.id.clone(),
            label: sys.name.clone(),
            sublabel: Some("[Software System]".into()),
            kind: sys.kind.clone(),
            tags: sys.tags.clone(),
            rect: Rect { x: x_system, y: sys_y, width: NODE_W * 1.2, height: NODE_H * 1.2 },
        });
    }

    // Edges: collapse container-level rels to system level
    for r in &rels_from {
        let from = if r.from.starts_with(&format!("{}.", scope_id)) { scope_id } else { &r.from };
        let to = if r.to.starts_with(&format!("{}.", scope_id)) { scope_id } else { &r.to };
        // Avoid duplicate edges
        if !edges.iter().any(|e: &LayoutEdge| e.from == from && e.to == to) {
            edges.push(LayoutEdge {
                from: from.to_string(),
                to: to.to_string(),
                label: r.label.clone(),
                technology: r.technology.clone(),
            });
        }
    }

    let max_actors = actor_ids.len().max(1);
    let total_h = y_start + max_actors as f64 * (NODE_H + V_GAP) + PADDING;
    let total_w = x_system + NODE_W * 1.2 + PADDING;

    Layout {
        width: total_w,
        height: total_h,
        title: view.title.clone().or_else(|| system.map(|s| format!("{} — System Context", s.name))),
        nodes,
        edges,
    }
}

/// Compute layout for a pipeline view.
pub fn layout_pipeline_view(model: &Model, view: &View) -> Layout {
    let pipeline_id = view.scope.as_deref().unwrap_or("");

    // Gather stages in this pipeline
    let stages: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.parent.as_deref() == Some(pipeline_id) && e.kind == ElementKind::Stage)
        .collect();

    // Topological sort by "needs" links
    let ordered = topo_sort_stages(&stages, &model.stage_links);

    let mut nodes = Vec::new();
    let mut x_off = PADDING;

    for stage in &ordered {
        // Gate before stage?
        let gate: Option<&Element> = model.elements.values().find(|e| {
            e.kind == ElementKind::Gate && e.parent.as_deref() == Some(&stage.id)
        });

        nodes.push(LayoutNode {
            id: stage.id.clone(),
            label: stage.name.clone(),
            sublabel: stage.properties.get("environment").map(|e| format!("[{}]", e)),
            kind: ElementKind::Stage,
            tags: stage.tags.clone(),
            rect: Rect { x: x_off, y: PADDING + 20.0, width: STAGE_W, height: STAGE_H },
        });

        x_off += STAGE_W + H_GAP;

        if let Some(g) = gate {
            nodes.push(LayoutNode {
                id: g.id.clone(),
                label: g.name.clone(),
                sublabel: None,
                kind: ElementKind::Gate,
                tags: g.tags.clone(),
                rect: Rect {
                    x: x_off - H_GAP / 2.0 - GATE_SIZE / 2.0,
                    y: PADDING + 20.0 + (STAGE_H - GATE_SIZE) / 2.0,
                    width: GATE_SIZE,
                    height: GATE_SIZE,
                },
            });
        }
    }

    // Edges: chain stages in order
    let mut edges = Vec::new();
    for link in &model.stage_links {
        if link.from.starts_with(pipeline_id) || link.to.starts_with(pipeline_id) {
            edges.push(LayoutEdge {
                from: link.from.clone(),
                to: link.to.clone(),
                label: String::new(),
                technology: None,
            });
        }
    }
    // Also chain sequentially for stages without explicit needs
    for i in 1..ordered.len() {
        let from = &ordered[i - 1].id;
        let to = &ordered[i].id;
        if !edges.iter().any(|e| e.from == *from && e.to == *to) {
            edges.push(LayoutEdge {
                from: from.clone(),
                to: to.clone(),
                label: String::new(),
                technology: None,
            });
        }
    }

    let total_w = x_off + PADDING;
    let total_h = PADDING * 2.0 + STAGE_H + 40.0;

    Layout {
        width: total_w,
        height: total_h,
        title: view.title.clone().or_else(|| {
            model.elements.get(pipeline_id).map(|p| format!("{} — Pipeline", p.name))
        }),
        nodes,
        edges,
    }
}

fn topo_sort_stages<'a>(stages: &[&'a Element], links: &[StageLink]) -> Vec<&'a Element> {
    // Simple: order by dependency depth
    let mut depth_map: HashMap<&str, usize> = HashMap::new();
    for s in stages {
        depth_map.insert(&s.id, 0);
    }

    // Iterate until stable
    for _ in 0..stages.len() {
        for link in links {
            if let (Some(&from_d), Some(_)) = (depth_map.get(link.from.as_str()), depth_map.get(link.to.as_str())) {
                let new_d = from_d + 1;
                let to_d = depth_map.entry(&link.to).or_insert(0);
                if new_d > *to_d {
                    *to_d = new_d;
                }
            }
        }
    }

    let mut sorted: Vec<&Element> = stages.to_vec();
    sorted.sort_by_key(|s| depth_map.get(s.id.as_str()).copied().unwrap_or(0));
    sorted
}

//! Forge layout engine — assigns positions to elements for SVG rendering.
//!
//! Dimensions match Structurizr-style proportions:
//!   - Structure elements: 240×120
//!   - Person elements: 240×160 (extra height for head+shoulders)
//!   - System elements: 280×140
//!   - Pipeline stages: 170×80
//!   - Gates: 70×70

use std::collections::HashSet;

use crate::model::*;

// ── Dimensions ──

const NODE_W: f64 = 240.0;
const NODE_H: f64 = 120.0;
const PERSON_W: f64 = 240.0;
const PERSON_H: f64 = 160.0;
const SYSTEM_W: f64 = 280.0;
const SYSTEM_H: f64 = 140.0;
const STAGE_W: f64 = 170.0;
const STAGE_H: f64 = 80.0;
const GATE_W: f64 = 70.0;
const GATE_H: f64 = 70.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 70.0;
const PAD: f64 = 60.0;
const TITLE_H: f64 = 50.0;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    pub sublabel: Option<String>,
    pub kind: ElementKind,
    pub tags: Vec<String>,
    pub rect: Rect,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

pub fn compute_layout(model: &Model, view: &View) -> Layout {
    match view.kind {
        ViewKind::SystemContext => layout_system_context(model, view),
        ViewKind::Container => layout_container(model, view),
        ViewKind::PipelineView => layout_pipeline(model, view),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────

fn dims_for(el: &Element) -> (f64, f64) {
    match el.kind {
        ElementKind::Person => (PERSON_W, PERSON_H),
        ElementKind::System => (SYSTEM_W, SYSTEM_H),
        _ => (NODE_W, NODE_H),
    }
}

fn make_node(el: &Element, x: f64, y: f64) -> LayoutNode {
    let (w, h) = dims_for(el);
    let sub = el.technology.as_ref().map(|t| format!("[{}]", t));
    LayoutNode {
        id: el.id.clone(),
        label: el.name.clone(),
        sublabel: sub,
        kind: el.kind,
        tags: el.tags.clone(),
        rect: Rect { x, y, w, h },
        description: el.description.clone(),
    }
}

// ─── System Context ──────────────────────────────────────────────

fn layout_system_context(model: &Model, view: &View) -> Layout {
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
            nodes.push(make_node(actor, PAD, y_cursor));
            y_cursor += dims_for(actor).1 + V_GAP;
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

// ─── Container View ──────────────────────────────────────────────

fn layout_container(model: &Model, view: &View) -> Layout {
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
    let ext_total_w: f64 = externals.iter().map(|e| dims_for(e).0).sum::<f64>()
        + H_GAP * (externals.len().saturating_sub(1) as f64);
    let cols: usize = 3;
    let grid_w = cols as f64 * NODE_W + (cols as f64 - 1.0) * H_GAP;
    let canvas_w = f64::max(ext_total_w, grid_w) + PAD * 2.0;
    let mut ext_x = (canvas_w - ext_total_w) / 2.0;

    for ext in &externals {
        let (ew, _) = dims_for(ext);
        nodes.push(make_node(ext, ext_x, TITLE_H));
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
        nodes.push(make_node(cont, x, y));
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

// ─── Pipeline View ───────────────────────────────────────────────

fn layout_pipeline(model: &Model, view: &View) -> Layout {
    let pipeline_id = view.scope.as_deref().unwrap_or("");
    let pipeline = model.elements.get(pipeline_id);

    let mut stages: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.parent.as_deref() == Some(pipeline_id) && e.kind == ElementKind::Stage)
        .collect();

    let ordered = topo_sort(&mut stages, &model.stage_links);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut x = PAD;
    let stage_cy = TITLE_H + 30.0;

    for stage in &ordered {
        nodes.push(LayoutNode {
            id: stage.id.clone(),
            label: stage.name.clone(),
            sublabel: stage.properties.get("environment").cloned(),
            kind: ElementKind::Stage,
            tags: stage.tags.clone(),
            rect: Rect {
                x,
                y: stage_cy,
                w: STAGE_W,
                h: STAGE_H,
            },
            description: None,
        });
        x += STAGE_W + H_GAP;

        // Gates
        let gates: Vec<&Element> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Gate && e.parent.as_deref() == Some(&stage.id))
            .collect();
        for g in gates {
            let gx = x - H_GAP / 2.0 - GATE_W / 2.0;
            let gy = stage_cy + (STAGE_H - GATE_H) / 2.0;
            nodes.push(LayoutNode {
                id: g.id.clone(),
                label: g.name.clone(),
                sublabel: None,
                kind: ElementKind::Gate,
                tags: g.tags.clone(),
                rect: Rect {
                    x: gx,
                    y: gy,
                    w: GATE_W,
                    h: GATE_H,
                },
                description: None,
            });
        }
    }

    for i in 1..ordered.len() {
        edges.push(LayoutEdge {
            frm: ordered[i - 1].id.clone(),
            to: ordered[i].id.clone(),
            label: String::new(),
            technology: None,
        });
    }

    let w = x + PAD;
    let h = TITLE_H + STAGE_H + 100.0;
    let title = view.title.clone().unwrap_or_else(|| {
        pipeline
            .map(|p| format!("{} — Pipeline", p.name))
            .unwrap_or_else(|| "Pipeline".into())
    });
    Layout {
        width: w,
        height: h,
        title: Some(title),
        nodes,
        edges,
    }
}

#[cfg(test)]
pub fn compute_layout_for_view(model: &Model, view_key: &str) -> Option<Layout> {
    model
        .views
        .iter()
        .find(|v| v.key == view_key)
        .map(|v| compute_layout(model, v))
}

fn topo_sort<'a>(stages: &mut [&'a Element], links: &[StageLink]) -> Vec<&'a Element> {
    use std::collections::HashMap;
    let mut depth: HashMap<&str, usize> = stages.iter().map(|s| (s.id.as_str(), 0)).collect();
    for _ in 0..stages.len() {
        for link in links {
            if let (Some(&d_frm), true) = (
                depth.get(link.frm.as_str()),
                depth.contains_key(link.to.as_str()),
            ) {
                let new = d_frm + 1;
                let d_to = depth.get_mut(link.to.as_str()).unwrap();
                if new > *d_to {
                    *d_to = new;
                }
            }
        }
    }
    stages.sort_by_key(|s| depth.get(s.id.as_str()).copied().unwrap_or(0));
    stages.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn payments_layout(view_key: &str) -> Layout {
        let text = include_str!("../examples/payments.forge");
        let model = parser::parse(text).unwrap();
        compute_layout_for_view(&model, view_key).expect("view should exist")
    }

    #[test]
    fn system_context_layout() {
        let lo = payments_layout("SystemContext");
        assert_eq!(
            lo.title.as_deref(),
            Some("Payment Platform — System Context")
        );
        // 1 actor (Customer) + 1 system (Payment Service)
        assert_eq!(lo.nodes.len(), 2);
        // collapsed relationships (customer->payments + internal self-edge)
        assert!(lo.edges.len() >= 1);
        assert!(lo.width > 0.0);
        assert!(lo.height > 0.0);
    }

    #[test]
    fn system_context_node_kinds() {
        let lo = payments_layout("SystemContext");
        assert!(lo.nodes.iter().any(|n| n.kind == ElementKind::Person));
        assert!(lo.nodes.iter().any(|n| n.kind == ElementKind::System));
    }

    #[test]
    fn container_layout() {
        let lo = payments_layout("Containers");
        assert_eq!(lo.title.as_deref(), Some("Payment Platform — Containers"));
        // 1 external (Customer) + 5 containers
        assert_eq!(lo.nodes.len(), 6);
        // edges between visible nodes
        assert!(lo.edges.len() >= 4);
    }

    #[test]
    fn container_layout_has_database() {
        let lo = payments_layout("Containers");
        assert!(lo
            .nodes
            .iter()
            .any(|n| n.tags.contains(&"database".to_string())));
    }

    #[test]
    fn pipeline_layout() {
        let lo = payments_layout("Pipeline");
        assert_eq!(lo.title.as_deref(), Some("Payment API — CI/CD Pipeline"));
        // 4 stages + 3 gates = 7 nodes
        let stage_count = lo
            .nodes
            .iter()
            .filter(|n| n.kind == ElementKind::Stage)
            .count();
        let gate_count = lo
            .nodes
            .iter()
            .filter(|n| n.kind == ElementKind::Gate)
            .count();
        assert_eq!(stage_count, 4);
        assert_eq!(gate_count, 3);
        // 3 edges between consecutive stages
        assert_eq!(lo.edges.len(), 3);
    }

    #[test]
    fn pipeline_topological_order() {
        let lo = payments_layout("Pipeline");
        let stages: Vec<&LayoutNode> = lo
            .nodes
            .iter()
            .filter(|n| n.kind == ElementKind::Stage)
            .collect();
        // Build should come before Security, Security before Staging, Staging before Prod
        let pos = |label: &str| stages.iter().position(|n| n.label.contains(label)).unwrap();
        assert!(pos("Build") < pos("Security"));
        assert!(pos("Security") < pos("Staging"));
        assert!(pos("Staging") < pos("Production"));
    }

    #[test]
    fn nodes_have_positive_dimensions() {
        let lo = payments_layout("Containers");
        for n in &lo.nodes {
            assert!(n.rect.w > 0.0, "node {} has zero width", n.id);
            assert!(n.rect.h > 0.0, "node {} has zero height", n.id);
        }
    }

    #[test]
    fn nodes_dont_overlap_in_pipeline() {
        let lo = payments_layout("Pipeline");
        let stages: Vec<&LayoutNode> = lo
            .nodes
            .iter()
            .filter(|n| n.kind == ElementKind::Stage)
            .collect();
        for i in 1..stages.len() {
            let prev_right = stages[i - 1].rect.x + stages[i - 1].rect.w;
            assert!(
                stages[i].rect.x > prev_right,
                "stages {} and {} overlap",
                stages[i - 1].id,
                stages[i].id
            );
        }
    }
}

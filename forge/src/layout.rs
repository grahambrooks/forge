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
#[allow(dead_code)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    pub sublabel: Option<String>,
    pub kind: ElementKind,
    pub tags: Vec<String>,
    pub rect: Rect,
    pub description: Option<String>,
    pub depth: usize,
    pub children_ids: Vec<String>,
    pub data_classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
    /// Step number for dynamic views. `None` for ordinary edges; `Some(n)`
    /// renders a circled step badge at the arrow midpoint.
    pub order: Option<u32>,
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
    let tm = TextMeasurer::new();
    compute_layout_with(model, view, &tm)
}

pub fn compute_layout_with(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    match view.kind {
        ViewKind::SystemContext => layout_system_context(model, view, tm),
        ViewKind::Container => layout_container(model, view, tm),
        ViewKind::PipelineView => layout_pipeline(model, view, tm),
        ViewKind::Deployment => layout_deployment(model, view, tm),
        ViewKind::TechStack => layout_tech_stack(model, view, tm),
        ViewKind::Branching => layout_branching(model, view, tm),
        ViewKind::Component => layout_component(model, view, tm),
        ViewKind::DataModel => layout_data_model(model, view, tm),
        ViewKind::TrustBoundaryView => layout_trust_boundary(model, view, tm),
        ViewKind::TeamMap => layout_team_map(model, view, tm),
        ViewKind::ApiCatalogView => layout_api_catalog(model, view, tm),
        ViewKind::EventFlowView => layout_event_flow(model, view, tm),
        ViewKind::Dynamic => layout_dynamic(model, view, tm),
        ViewKind::Composite => layout_composite(model, view, tm),
    }
}

// ─── Text Measurement ────────────────────────────────────────────

use crate::text::*;

const BOX_PAD_X: f64 = 20.0;
const BOX_PAD_Y: f64 = 14.0;
const MIN_NODE_W: f64 = 160.0;
const MIN_NODE_H: f64 = 60.0;

/// Measure the required (width, height) for a structural element.
fn measure_element(el: &Element, tm: &TextMeasurer) -> (f64, f64) {
    let sub = el.technology.as_ref().map(|t| format!("[{}]", t));

    // Compute minimum width from the widest text line
    let name_w = tm.measure(&el.name, &FONT_NAME);
    let kind_label = kind_label_for(el.kind);
    let kind_w = kind_label
        .map(|k| tm.measure(&format!("[{}]", k), &FONT_KIND))
        .unwrap_or(0.0);
    let tech_w = sub
        .as_ref()
        .map(|s| tm.measure(s, &FONT_TECH))
        .unwrap_or(0.0);

    // For description, we'll wrap it — but need a target width first
    let base_w = name_w.max(kind_w).max(tech_w) + BOX_PAD_X * 2.0;
    // Clamp width: at least min, at most a reasonable max
    let target_w = base_w.clamp(MIN_NODE_W, 320.0);
    let desc_max_w = target_w - BOX_PAD_X * 2.0;

    let mut line_count = 1; // name
    if kind_label.is_some() {
        line_count += 1;
    }
    if let Some(ref desc) = el.description {
        let wrapped = tm.wrap(desc, desc_max_w, &FONT_DESC);
        line_count += wrapped.len().min(3); // max 3 lines of description
    }
    if sub.is_some() {
        line_count += 1;
    }

    let content_h = line_count as f64 * FONT_NAME.line_height;
    let h = (content_h + BOX_PAD_Y * 2.0).max(MIN_NODE_H);

    // For person elements, add head+shoulders space
    let (w, h) = match el.kind {
        ElementKind::Person => (target_w.max(PERSON_W), h + 62.0), // silhouette
        ElementKind::System => (target_w.max(SYSTEM_W), h.max(SYSTEM_H)),
        _ => (target_w, h),
    };

    (w, h)
}

fn kind_label_for(kind: ElementKind) -> Option<&'static str> {
    match kind {
        ElementKind::Person => Some("Person"),
        ElementKind::System => Some("Software System"),
        ElementKind::Container => Some("Container"),
        ElementKind::Component => Some("Component"),
        _ => None,
    }
}

// ─── Helpers ──────────────────────────────────────────────────────

fn dims_for(el: &Element, tm: &TextMeasurer) -> (f64, f64) {
    measure_element(el, tm)
}

fn make_node(el: &Element, x: f64, y: f64, tm: &TextMeasurer) -> LayoutNode {
    let (w, h) = dims_for(el, tm);
    let sub = el.technology.as_ref().map(|t| format!("[{}]", t));
    LayoutNode {
        id: el.id.clone(),
        label: el.name.clone(),
        sublabel: sub,
        kind: el.kind,
        tags: el.tags.clone(),
        rect: Rect { x, y, w, h },
        description: el.description.clone(),
        depth: 0,
        children_ids: Vec::new(),
        data_classes: el.data_classes.clone(),
    }
}

// ─── System Context ──────────────────────────────────────────────

fn layout_system_context(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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

// ─── Container View ──────────────────────────────────────────────

fn layout_container(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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

// ─── Pipeline View ───────────────────────────────────────────────

fn layout_pipeline(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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
        // Auto-size stage based on name + optional environment sublabel
        let env_sub = stage.properties.get("environment");
        let name_w = tm.measure(&stage.name, &FONT_NAME);
        let sub_w = env_sub
            .map(|s| tm.measure(&format!("[{}]", s), &FONT_TECH))
            .unwrap_or(0.0);
        let sw = name_w.max(sub_w).max(STAGE_W - BOX_PAD_X * 2.0) + BOX_PAD_X * 2.0;
        let line_count = if env_sub.is_some() { 2 } else { 1 };
        let sh = (line_count as f64 * FONT_NAME.line_height + BOX_PAD_Y * 2.0).max(STAGE_H);

        nodes.push(LayoutNode {
            id: stage.id.clone(),
            label: stage.name.clone(),
            sublabel: env_sub.cloned(),
            kind: ElementKind::Stage,
            tags: stage.tags.clone(),
            rect: Rect {
                x,
                y: stage_cy,
                w: sw,
                h: sh,
            },
            description: None,
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });
        x += sw + H_GAP;

        // Gates — auto-size based on name
        let gates: Vec<&Element> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Gate && e.parent.as_deref() == Some(&stage.id))
            .collect();
        for g in gates {
            let gate_text_w = tm.measure(&g.name, &FONT_GATE);
            let gw = (gate_text_w + 30.0).max(GATE_W);
            let gh = gw; // keep diamond square
            let gx = x - H_GAP / 2.0 - gw / 2.0;
            let gy = stage_cy + (sh - gh) / 2.0;
            nodes.push(LayoutNode {
                id: g.id.clone(),
                label: g.name.clone(),
                sublabel: None,
                kind: ElementKind::Gate,
                tags: g.tags.clone(),
                rect: Rect {
                    x: gx,
                    y: gy,
                    w: gw,
                    h: gh,
                },
                description: None,
                depth: 0,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
        }
    }

    for i in 1..ordered.len() {
        edges.push(LayoutEdge {
            frm: ordered[i - 1].id.clone(),
            to: ordered[i].id.clone(),
            label: String::new(),
            technology: None,
            order: None,
        });
    }

    let w = x + PAD;
    let max_node_h = nodes.iter().map(|n| n.rect.h).fold(STAGE_H, f64::max);
    let h = TITLE_H + max_node_h + 100.0;
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

// ─── Deployment View ─────────────────────────────────────────────

const DEPLOY_PAD: f64 = 20.0;
const DEPLOY_HEADER: f64 = 36.0;
const DEPLOY_INSTANCE_W: f64 = 200.0;
const DEPLOY_INSTANCE_H: f64 = 70.0;
const DEPLOY_GAP: f64 = 16.0;

fn layout_deployment(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let env_id = view.scope.as_deref().unwrap_or("");

    // Find top-level deployment nodes for this environment
    let top_nodes: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| {
            e.kind == ElementKind::DeploymentNode
                && e.properties.get("environment").map(|s| s.as_str()) == Some(env_id)
                && e.parent.as_deref() == Some(env_id)
        })
        .collect();

    let mut nodes = Vec::new();
    let mut x = PAD;
    let y = TITLE_H + 10.0;

    for top in &top_nodes {
        let (w, _h) = measure_deploy_node(model, top);
        layout_deploy_node(model, top, x, y, 0, &mut nodes);
        x += w + DEPLOY_GAP * 2.0;
    }

    let canvas_w = x + PAD;
    let canvas_h = nodes
        .iter()
        .map(|n| n.rect.y + n.rect.h)
        .fold(200.0_f64, f64::max)
        + PAD
        + 40.0;

    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Deployment", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges: Vec::new(),
    }
}

/// Recursively measure a deployment node to determine its required (w, h).
fn measure_deploy_node(model: &Model, el: &Element) -> (f64, f64) {
    let child_nodes: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::DeploymentNode && e.parent.as_deref() == Some(&el.id))
        .collect();

    let instances: Vec<&str> = el
        .properties
        .get("container_instances")
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    if child_nodes.is_empty() && instances.is_empty() {
        return (
            DEPLOY_INSTANCE_W + DEPLOY_PAD * 2.0,
            DEPLOY_HEADER + DEPLOY_PAD * 2.0,
        );
    }

    // Layout children horizontally
    let mut content_w = 0.0_f64;
    let mut content_h = 0.0_f64;

    for child in &child_nodes {
        let (cw, ch) = measure_deploy_node(model, child);
        content_w += cw + DEPLOY_GAP;
        content_h = content_h.max(ch);
    }

    // Layout instances in a row below child nodes
    let inst_row_w = instances.len() as f64 * (DEPLOY_INSTANCE_W + DEPLOY_GAP) - DEPLOY_GAP;
    let inst_row_h = if instances.is_empty() {
        0.0
    } else {
        DEPLOY_INSTANCE_H + DEPLOY_GAP
    };

    content_w = content_w.max(inst_row_w);
    if !instances.is_empty() && !child_nodes.is_empty() {
        content_h += inst_row_h;
    } else if !instances.is_empty() {
        content_h = inst_row_h;
    }

    if content_w > 0.0 {
        content_w -= DEPLOY_GAP; // remove trailing gap
    }

    let total_w = content_w + DEPLOY_PAD * 2.0;
    let total_h = DEPLOY_HEADER + content_h + DEPLOY_PAD;

    (total_w.max(DEPLOY_INSTANCE_W + DEPLOY_PAD * 2.0), total_h)
}

/// Recursively place deployment nodes and their children.
fn layout_deploy_node(
    model: &Model,
    el: &Element,
    x: f64,
    y: f64,
    depth: usize,
    nodes: &mut Vec<LayoutNode>,
) {
    let (w, h) = measure_deploy_node(model, el);

    let child_elements: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::DeploymentNode && e.parent.as_deref() == Some(&el.id))
        .collect();

    let instances: Vec<&str> = el
        .properties
        .get("container_instances")
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let child_ids: Vec<String> = child_elements
        .iter()
        .map(|e| e.id.clone())
        .chain(instances.iter().map(|s| format!("{}._inst_{}", el.id, s)))
        .collect();

    let sublabel = el.technology.as_ref().map(|t| format!("[{}]", t));
    nodes.push(LayoutNode {
        id: el.id.clone(),
        label: el.name.clone(),
        sublabel,
        kind: ElementKind::DeploymentNode,
        tags: el.tags.clone(),
        rect: Rect { x, y, w, h },
        description: el.description.clone(),
        depth,
        children_ids: child_ids,
        data_classes: el.data_classes.clone(),
    });

    // Place child nodes horizontally
    let mut cx = x + DEPLOY_PAD;
    let cy = y + DEPLOY_HEADER;

    for child in &child_elements {
        let (cw, _ch) = measure_deploy_node(model, child);
        layout_deploy_node(model, child, cx, cy, depth + 1, nodes);
        cx += cw + DEPLOY_GAP;
    }

    // Place container instances
    let inst_y = if child_elements.is_empty() {
        cy
    } else {
        // Below child nodes
        let max_child_bottom = child_elements
            .iter()
            .map(|c| {
                let (_, ch) = measure_deploy_node(model, c);
                cy + ch
            })
            .fold(cy, f64::max);
        max_child_bottom + DEPLOY_GAP
    };

    let mut ix = x + DEPLOY_PAD;
    for inst_ref in &instances {
        let container = model.elements.get(*inst_ref);
        let inst_label = container.map(|c| c.name.as_str()).unwrap_or(inst_ref);
        let inst_sub = container.and_then(|c| c.technology.as_ref().map(|t| format!("[{}]", t)));
        let inst_kind = container.map(|c| c.kind).unwrap_or(ElementKind::Container);
        let inst_tags = container.map(|c| c.tags.clone()).unwrap_or_default();
        let inst_dc = container
            .map(|c| c.data_classes.clone())
            .unwrap_or_default();

        nodes.push(LayoutNode {
            id: format!("{}._inst_{}", el.id, inst_ref),
            label: inst_label.to_string(),
            sublabel: inst_sub,
            kind: inst_kind,
            tags: inst_tags,
            rect: Rect {
                x: ix,
                y: inst_y,
                w: DEPLOY_INSTANCE_W,
                h: DEPLOY_INSTANCE_H,
            },
            description: None,
            depth: depth + 1,
            children_ids: Vec::new(),
            data_classes: inst_dc,
        });
        ix += DEPLOY_INSTANCE_W + DEPLOY_GAP;
    }
}

// ─── Tech Stack View ─────────────────────────────────────────────

const TECH_CARD_W: f64 = 180.0;
const TECH_CARD_H: f64 = 60.0;
const TECH_GAP: f64 = 14.0;
const TECH_CAT_PAD: f64 = 16.0;
const TECH_CAT_HEADER: f64 = 32.0;
const TECH_COLS: usize = 4;

fn layout_tech_stack(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut y = TITLE_H + 10.0;

    let grid_w = TECH_COLS as f64 * (TECH_CARD_W + TECH_GAP) - TECH_GAP + TECH_CAT_PAD * 2.0;

    for cat in &model.tech_stack {
        let rows = cat.entries.len().div_ceil(TECH_COLS);
        let cat_h = TECH_CAT_HEADER + rows as f64 * (TECH_CARD_H + TECH_GAP) + TECH_CAT_PAD;

        // Category background node
        nodes.push(LayoutNode {
            id: format!("_techcat_{}", cat.name.to_lowercase().replace(' ', "-")),
            label: cat.name.clone(),
            sublabel: None,
            kind: ElementKind::DeploymentNode, // reuse for nested box rendering
            tags: vec!["tech-category".into()],
            rect: Rect {
                x: PAD,
                y,
                w: grid_w,
                h: cat_h,
            },
            description: None,
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });

        // Tech entry cards
        for (i, entry) in cat.entries.iter().enumerate() {
            let col = i % TECH_COLS;
            let row = i / TECH_COLS;
            let ex = PAD + TECH_CAT_PAD + col as f64 * (TECH_CARD_W + TECH_GAP);
            let ey = y + TECH_CAT_HEADER + row as f64 * (TECH_CARD_H + TECH_GAP);

            let sublabel = entry
                .version
                .as_ref()
                .map(|v| format!("v{}", v))
                .or(entry.purpose.as_ref().cloned());

            nodes.push(LayoutNode {
                id: format!(
                    "_tech_{}_{}",
                    cat.name.to_lowercase().replace(' ', "-"),
                    entry.name.to_lowercase().replace(' ', "-")
                ),
                label: entry.name.clone(),
                sublabel,
                kind: ElementKind::Container, // renders as a rounded box
                tags: vec!["tech-entry".into()],
                rect: Rect {
                    x: ex,
                    y: ey,
                    w: TECH_CARD_W,
                    h: TECH_CARD_H,
                },
                description: entry.purpose.clone(),
                depth: 1,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
        }

        y += cat_h + TECH_GAP;
    }

    let canvas_w = grid_w + PAD * 2.0;
    let canvas_h = y + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Tech Stack", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges: Vec::new(),
    }
}

// ─── Branching Strategy View ─────────────────────────────────────

// ─── Branching (gitgraph-style) ─────────────────────────────────
// Renders a left-to-right timeline with parallel branch lines,
// commit dots, and curved branch/merge connections — similar to
// Mermaid's gitgraph diagrams.

const BRANCH_LANE_GAP: f64 = 40.0; // vertical spacing between branch lanes
const BRANCH_LABEL_W: f64 = 120.0; // space reserved for branch name labels
const COMMIT_SPACING: f64 = 60.0; // horizontal distance between commit dots
const COMMIT_R: f64 = 8.0; // commit dot radius

fn layout_branching(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let strategy_id = view.scope.as_deref().unwrap_or("");

    let branches: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| {
            e.kind == ElementKind::Branch
                && e.properties.get("strategy").map(|s| s.as_str()) == Some(strategy_id)
        })
        .collect();

    // Sort: trunk first, then others
    let mut sorted = branches.clone();
    sorted.sort_by_key(|b| {
        if b.properties.contains_key("protection") {
            0
        } else {
            1
        }
    });

    // Assign each branch a lane index (row) and a color index
    let num_branches = sorted.len();
    // Commits per branch: trunk gets more (it's the long-lived line),
    // feature branches get fewer (short-lived)
    let trunk_commits = 7usize;
    let feature_commits = 3usize;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let timeline_x = PAD + BRANCH_LABEL_W;
    let top_y = TITLE_H + 20.0;

    for (lane, branch) in sorted.iter().enumerate() {
        let is_trunk = branch.properties.contains_key("protection");
        let num_commits = if is_trunk {
            trunk_commits
        } else {
            feature_commits
        };
        let lane_y = top_y + lane as f64 * BRANCH_LANE_GAP;

        let mut tags = branch.tags.clone();
        if is_trunk {
            tags.push("trunk".into());
        }
        // Encode the lane index and color for the renderer
        tags.push(format!("gitgraph-lane-{}", lane));

        // Sublabel encodes commit count and lane_y for the renderer
        let sublabel = Some(format!(
            "commits={};lane_y={:.0};timeline_x={:.0};is_trunk={}",
            num_commits, lane_y, timeline_x, is_trunk
        ));

        // The node rect spans the full timeline width
        let timeline_w = (trunk_commits as f64) * COMMIT_SPACING;
        nodes.push(LayoutNode {
            id: branch.id.clone(),
            label: branch.name.clone(),
            sublabel,
            kind: ElementKind::Branch,
            tags,
            rect: Rect {
                x: PAD,
                y: lane_y - COMMIT_R,
                w: BRANCH_LABEL_W + timeline_w + PAD,
                h: COMMIT_R * 2.0,
            },
            description: branch.description.clone(),
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });
    }

    // Edges for branches-from / merges-into (renderer draws curves)
    for branch in &sorted {
        if let Some(from) = branch.properties.get("branches-from") {
            edges.push(LayoutEdge {
                frm: from.clone(),
                to: branch.id.clone(),
                label: String::new(),
                technology: None,
                order: None,
            });
        }
        if let Some(into) = branch.properties.get("merges-into") {
            edges.push(LayoutEdge {
                frm: branch.id.clone(),
                to: into.clone(),
                label: String::new(),
                technology: None,
                order: None,
            });
        }
    }

    let timeline_w = (trunk_commits as f64) * COMMIT_SPACING;
    let canvas_w = PAD + BRANCH_LABEL_W + timeline_w + PAD * 2.0;
    let canvas_h = top_y + (num_branches as f64) * BRANCH_LANE_GAP + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Branching Strategy", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges,
    }
}

// ─── Component View ──────────────────────────────────────────────

fn layout_component(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    // Same approach as container view, but scoped to components within a container
    let scope_id = view.scope.as_deref().unwrap_or("");
    let container = model.elements.get(scope_id);

    let components: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Component)
        .collect();

    let component_ids: std::collections::HashSet<String> =
        components.iter().map(|c| c.id.clone()).collect();
    let rels = model.relationships_involving(&component_ids);

    // External elements (things outside this container that connect to components)
    let mut ext_ids: Vec<String> = Vec::new();
    for r in &rels {
        for eid in [&r.frm, &r.to] {
            if !component_ids.contains(eid) && eid.as_str() != scope_id && !ext_ids.contains(eid) {
                ext_ids.push(eid.clone());
            }
        }
    }
    let externals: Vec<&Element> = ext_ids
        .iter()
        .filter_map(|eid| model.elements.get(eid))
        .collect();

    let mut nodes = Vec::new();

    // External elements on top row
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

    // Components in a grid
    let y_start = TITLE_H
        + if !externals.is_empty() {
            NODE_H + V_GAP * 1.3
        } else {
            0.0
        };
    let grid_start_x = (canvas_w - grid_w) / 2.0;

    for (i, comp) in components.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let x = grid_start_x + col as f64 * (NODE_W + H_GAP);
        let y = y_start + row as f64 * (NODE_H + V_GAP);
        nodes.push(make_node(comp, x, y, tm));
    }

    let all_ids: std::collections::HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
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
        container
            .map(|c| format!("{} — Components", c.name))
            .unwrap_or_else(|| "Components".into())
    });
    Layout {
        width: f64::max(max_x, canvas_w),
        height: max_y,
        title: Some(title),
        nodes,
        edges,
    }
}

// ─── Data Model View ─────────────────────────────────────────────

const ENTITY_W: f64 = 220.0;
const ENTITY_HEADER_H: f64 = 32.0;
const ENTITY_FIELD_H: f64 = 20.0;
const ENTITY_PAD: f64 = 10.0;
const ENTITY_GAP: f64 = 60.0;
const ENTITY_COLS: usize = 3;

fn layout_data_model(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Measure entity widths first so we can compute proper column layout
    let entity_data: Vec<(String, String, f64, f64)> = model
        .data_entities
        .iter()
        .map(|entity| {
            let fields_desc = entity
                .fields
                .iter()
                .map(|f| {
                    let c = if f.constraints.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", f.constraints.join(", "))
                    };
                    format!("{}: {}{}", f.name, f.field_type, c)
                })
                .collect::<Vec<_>>()
                .join("\n");
            // Measure width using actual font metrics
            let entity_name_w = tm.measure(&entity.name, &FONT_NAME);
            let owner_w = entity
                .owner
                .as_ref()
                .map(|o| {
                    let oname = model.elements.get(o).map(|e| e.name.as_str()).unwrap_or(o);
                    tm.measure(&format!("Owner: {}", oname), &FONT_ENTITY_SUB)
                })
                .unwrap_or(0.0);
            let header_w = entity_name_w + owner_w + 30.0;
            let max_field_w = entity
                .fields
                .iter()
                .map(|f| {
                    let fname_w = tm.measure(&f.name, &FONT_ENTITY_FIELD);
                    let constraints = if f.constraints.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", f.constraints.join(", "))
                    };
                    let ftype_w = tm.measure(
                        &format!("{}{}", f.field_type, constraints),
                        &FONT_ENTITY_TYPE,
                    );
                    fname_w + ftype_w + 40.0
                })
                .fold(0.0_f64, f64::max);
            let w = entity_name_w
                .max(header_w)
                .max(max_field_w)
                .max(ENTITY_W - 30.0)
                + 30.0;
            let h =
                ENTITY_HEADER_H + entity.fields.len() as f64 * ENTITY_FIELD_H + ENTITY_PAD * 2.0;
            (entity.name.clone(), fields_desc, w, h)
        })
        .collect();

    let max_entity_w = entity_data
        .iter()
        .map(|(_, _, w, _)| *w)
        .fold(ENTITY_W, f64::max);

    for (i, entity) in model.data_entities.iter().enumerate() {
        let col = i % ENTITY_COLS;
        let row = i / ENTITY_COLS;
        let x = PAD + col as f64 * (max_entity_w + ENTITY_GAP);
        let (_, ref fields_desc, _, h) = entity_data[i];
        let y = TITLE_H + 10.0 + row as f64 * (200.0 + ENTITY_GAP);

        let sublabel = entity.owner.as_ref().map(|o| {
            let owner_name = model.elements.get(o).map(|e| e.name.as_str()).unwrap_or(o);
            format!("Owner: {}", owner_name)
        });

        nodes.push(LayoutNode {
            id: format!("_entity_{}", entity.name.to_lowercase().replace(' ', "-")),
            label: entity.name.clone(),
            sublabel,
            kind: ElementKind::Container,
            tags: vec!["data-entity".into()],
            rect: Rect {
                x,
                y,
                w: max_entity_w,
                h,
            },
            description: Some(fields_desc.clone()),
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });
    }

    for rel in &model.data_relations {
        let from_id = format!(
            "_entity_{}",
            rel.from_entity.to_lowercase().replace(' ', "-")
        );
        let to_id = format!("_entity_{}", rel.to_entity.to_lowercase().replace(' ', "-"));
        edges.push(LayoutEdge {
            frm: from_id,
            to: to_id,
            label: format!("{} [{}]", rel.label, rel.cardinality),
            technology: None,
            order: None,
        });
    }

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
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Data Model", model.name));

    Layout {
        width: max_x,
        height: max_y,
        title: Some(title),
        nodes,
        edges,
    }
}

// ─── Trust Boundary View ─────────────────────────────────────────

const BOUNDARY_PAD: f64 = 24.0;
const BOUNDARY_HEADER: f64 = 34.0;
const BOUNDARY_MEMBER_W: f64 = 180.0;
const BOUNDARY_MEMBER_H: f64 = 60.0;
const BOUNDARY_MEMBER_GAP: f64 = 16.0;

fn layout_trust_boundary(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut y = TITLE_H + 10.0;

    let max_members = model
        .trust_boundaries
        .iter()
        .map(|b| b.members.len())
        .max()
        .unwrap_or(1);
    let zone_w = (max_members as f64 * (BOUNDARY_MEMBER_W + BOUNDARY_MEMBER_GAP)
        - BOUNDARY_MEMBER_GAP
        + BOUNDARY_PAD * 2.0)
        .max(400.0);

    for boundary in &model.trust_boundaries {
        let max_member_h = boundary
            .members
            .iter()
            .map(|id| {
                let kind = model
                    .elements
                    .get(id)
                    .map(|e| e.kind)
                    .unwrap_or(ElementKind::Container);
                if kind == ElementKind::Person {
                    BOUNDARY_MEMBER_H + 62.0
                } else {
                    BOUNDARY_MEMBER_H
                }
            })
            .fold(BOUNDARY_MEMBER_H, f64::max);
        let zone_h = BOUNDARY_HEADER + max_member_h + BOUNDARY_PAD * 2.0;
        let mut tags = vec!["trust-zone".into()];
        tags.push(format!("trust-{}", boundary.level));

        nodes.push(LayoutNode {
            id: format!("_zone_{}", boundary.name.to_lowercase().replace(' ', "-")),
            label: format!("{} [{}]", boundary.name, boundary.level),
            sublabel: None,
            kind: ElementKind::DeploymentNode,
            tags,
            rect: Rect {
                x: PAD,
                y,
                w: zone_w,
                h: zone_h,
            },
            description: None,
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });

        let mut mx = PAD + BOUNDARY_PAD;
        let my = y + BOUNDARY_HEADER;
        for member_id in &boundary.members {
            let member = model.elements.get(member_id);
            let label = member.map(|e| e.name.as_str()).unwrap_or(member_id);
            let sublabel = member.and_then(|e| e.technology.as_ref().map(|t| format!("[{}]", t)));
            let kind = member.map(|e| e.kind).unwrap_or(ElementKind::Container);
            let member_tags = member.map(|e| e.tags.clone()).unwrap_or_default();

            // Person elements need more height for head+shoulders silhouette
            let member_h = if kind == ElementKind::Person {
                BOUNDARY_MEMBER_H + 62.0
            } else {
                BOUNDARY_MEMBER_H
            };

            nodes.push(LayoutNode {
                id: format!(
                    "_zone_{}._m_{}",
                    boundary.name.to_lowercase().replace(' ', "-"),
                    member_id.replace('.', "-")
                ),
                label: label.to_string(),
                sublabel,
                kind,
                tags: member_tags,
                rect: Rect {
                    x: mx,
                    y: my,
                    w: BOUNDARY_MEMBER_W,
                    h: member_h,
                },
                description: None,
                depth: 1,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
            mx += BOUNDARY_MEMBER_W + BOUNDARY_MEMBER_GAP;
        }
        y += zone_h + DEPLOY_GAP;
    }

    let canvas_w = zone_w + PAD * 2.0;
    let canvas_h = y + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Trust Boundaries", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges: Vec::new(),
    }
}

// ─── Team Map View ───────────────────────────────────────────────

const TEAM_W: f64 = 500.0;
const TEAM_HEADER_H: f64 = 36.0;
const TEAM_MEMBER_H: f64 = 28.0;
const TEAM_PAD: f64 = 14.0;

fn layout_team_map(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut y = TITLE_H + 10.0;

    for team in &model.teams {
        let team_h = TEAM_HEADER_H + team.owns.len() as f64 * TEAM_MEMBER_H + TEAM_PAD * 2.0;

        let owns_desc = team
            .owns
            .iter()
            .map(|id| {
                model
                    .elements
                    .get(id)
                    .map(|e| e.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect::<Vec<_>>()
            .join(", ");

        nodes.push(LayoutNode {
            id: format!("_team_{}", team.name.to_lowercase().replace(' ', "-")),
            label: team.name.clone(),
            sublabel: team.contact.clone(),
            kind: ElementKind::DeploymentNode,
            tags: vec!["team".into()],
            rect: Rect {
                x: PAD,
                y,
                w: TEAM_W,
                h: team_h,
            },
            description: Some(owns_desc),
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });

        let mut oy = y + TEAM_HEADER_H;
        for owned_id in &team.owns {
            let el = model.elements.get(owned_id);
            let label = el.map(|e| e.name.as_str()).unwrap_or(owned_id);
            let kind = el.map(|e| e.kind).unwrap_or(ElementKind::Container);
            let sublabel = el.and_then(|e| e.technology.as_ref().map(|t| format!("[{}]", t)));

            nodes.push(LayoutNode {
                id: format!(
                    "_team_{}._o_{}",
                    team.name.to_lowercase().replace(' ', "-"),
                    owned_id.replace('.', "-")
                ),
                label: label.to_string(),
                sublabel,
                kind,
                tags: Vec::new(),
                rect: Rect {
                    x: PAD + TEAM_PAD,
                    y: oy,
                    w: TEAM_W - TEAM_PAD * 2.0,
                    h: TEAM_MEMBER_H,
                },
                description: None,
                depth: 1,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
            oy += TEAM_MEMBER_H;
        }
        y += team_h + DEPLOY_GAP;
    }

    let canvas_w = TEAM_W + PAD * 2.0;
    let canvas_h = y + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Team Ownership", model.name));

    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges: Vec::new(),
    }
}

// ─── API Catalog View (stub) ────────────────────────────────────

fn layout_api_catalog(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut y = TITLE_H + 10.0;
    let card_w = 400.0;
    let card_h = 28.0;
    let group_pad = 16.0;
    let group_header = 34.0;

    for catalog in &model.api_catalogs {
        let container_name = model
            .elements
            .get(&catalog.container)
            .map(|e| e.name.as_str())
            .unwrap_or(&catalog.container);
        let group_h = group_header + catalog.endpoints.len() as f64 * card_h + group_pad * 2.0;

        // Container group box
        nodes.push(LayoutNode {
            id: format!("_api_{}", catalog.container.replace('.', "-")),
            label: container_name.to_string(),
            sublabel: None,
            kind: ElementKind::DeploymentNode,
            tags: vec!["api-group".into()],
            rect: Rect {
                x: PAD,
                y,
                w: card_w + group_pad * 2.0,
                h: group_h,
            },
            description: None,
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });

        // Endpoint cards
        let mut ey = y + group_header;
        for ep in &catalog.endpoints {
            let label = format!("{} {}", ep.method, ep.path);
            nodes.push(LayoutNode {
                id: format!(
                    "_ep_{}_{}",
                    catalog.container.replace('.', "-"),
                    ep.path.replace('/', "-")
                ),
                label,
                sublabel: ep.description.clone(),
                kind: ElementKind::Container,
                tags: vec!["api-endpoint".into()],
                rect: Rect {
                    x: PAD + group_pad,
                    y: ey,
                    w: card_w,
                    h: card_h,
                },
                description: None,
                depth: 1,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
            ey += card_h;
        }
        y += group_h + DEPLOY_GAP;
    }

    let canvas_w = card_w + group_pad * 2.0 + PAD * 2.0;
    let canvas_h = y + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — API Catalog", model.name));
    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges: Vec::new(),
    }
}

// ─── Event Flow View ─────────────────────────────────────────────

fn layout_event_flow(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let topic_w = 200.0;
    let topic_h = 50.0;
    let actor_w = 160.0;
    let actor_h = 50.0;
    let mut y = TITLE_H + 10.0;

    for flow in &model.event_flows {
        let topic_x = PAD + actor_w + H_GAP;
        let topic_id = format!(
            "_topic_{}",
            flow.name.replace(|c: char| !c.is_alphanumeric(), "-")
        );

        // Topic node (center)
        nodes.push(LayoutNode {
            id: topic_id.clone(),
            label: flow.topic.as_deref().unwrap_or(&flow.name).to_string(),
            sublabel: flow.description.clone(),
            kind: ElementKind::Stage,
            tags: vec!["event-topic".into()],
            rect: Rect {
                x: topic_x,
                y,
                w: topic_w,
                h: topic_h,
            },
            description: None,
            depth: 0,
            children_ids: Vec::new(),
            data_classes: Vec::new(),
        });

        // Publishers (left)
        let mut py = y;
        for pub_id in &flow.publishers {
            let pub_name = model
                .elements
                .get(pub_id)
                .map(|e| e.name.as_str())
                .unwrap_or(pub_id);
            let pid = format!("{}_pub_{}", topic_id, pub_id.replace('.', "-"));
            nodes.push(LayoutNode {
                id: pid.clone(),
                label: pub_name.to_string(),
                sublabel: Some("publisher".into()),
                kind: ElementKind::Container,
                tags: Vec::new(),
                rect: Rect {
                    x: PAD,
                    y: py,
                    w: actor_w,
                    h: actor_h,
                },
                description: None,
                depth: 0,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
            edges.push(LayoutEdge {
                frm: pid,
                to: topic_id.clone(),
                label: "publishes".into(),
                technology: None,
                order: None,
            });
            py += actor_h + 10.0;
        }

        // Subscribers (right)
        let sub_x = topic_x + topic_w + H_GAP;
        let mut sy = y;
        for sub_id in &flow.subscribers {
            let sub_name = model
                .elements
                .get(sub_id)
                .map(|e| e.name.as_str())
                .unwrap_or(sub_id);
            let sid = format!("{}_sub_{}", topic_id, sub_id.replace('.', "-"));
            nodes.push(LayoutNode {
                id: sid.clone(),
                label: sub_name.to_string(),
                sublabel: Some("subscriber".into()),
                kind: ElementKind::Container,
                tags: Vec::new(),
                rect: Rect {
                    x: sub_x,
                    y: sy,
                    w: actor_w,
                    h: actor_h,
                },
                description: None,
                depth: 0,
                children_ids: Vec::new(),
                data_classes: Vec::new(),
            });
            edges.push(LayoutEdge {
                frm: topic_id.clone(),
                to: sid,
                label: "delivers".into(),
                technology: None,
                order: None,
            });
            sy += actor_h + 10.0;
        }

        let row_h = f64::max(py, sy) - y;
        y += row_h.max(topic_h) + V_GAP;
    }

    let canvas_w = PAD + actor_w + H_GAP + topic_w + H_GAP + actor_w + PAD;
    let canvas_h = y + PAD;
    let title = view
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — Event Flows", model.name));
    Layout {
        width: canvas_w,
        height: canvas_h,
        title: Some(title),
        nodes,
        edges,
    }
}

// ─── Dynamic View ────────────────────────────────────────────────

/// A dynamic view looks identical to a container view in structure —
/// the same scope, the same nodes, the same layout. What's different
/// is that relationships with an `order` field render with a circled
/// step number on the arrow. Ordering is static metadata, not a
/// layout concern; the renderer picks it up directly.
fn layout_dynamic(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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
fn layout_composite(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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

    #[test]
    fn deployment_layout() {
        let lo = payments_layout("Deployment");
        assert!(lo.title.as_deref().unwrap().contains("Deployment"));
        // Should have deployment nodes + container instances
        let deploy_nodes = lo
            .nodes
            .iter()
            .filter(|n| n.kind == ElementKind::DeploymentNode)
            .count();
        assert!(
            deploy_nodes >= 5,
            "expected >= 5 deployment nodes, got {}",
            deploy_nodes
        );
        // Should have container instances nested inside
        let instances = lo
            .nodes
            .iter()
            .filter(|n| n.kind != ElementKind::DeploymentNode && n.depth > 0)
            .count();
        assert!(
            instances >= 3,
            "expected >= 3 container instances, got {}",
            instances
        );
        assert!(lo.width > 0.0);
        assert!(lo.height > 0.0);
    }
}

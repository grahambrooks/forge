use super::*;

// ─── Deployment View ─────────────────────────────────────────────

const DEPLOY_PAD: f64 = 20.0;
const DEPLOY_HEADER: f64 = 36.0;
const DEPLOY_INSTANCE_W: f64 = 200.0;
const DEPLOY_INSTANCE_H: f64 = 70.0;
pub(super) const DEPLOY_GAP: f64 = 16.0;

pub(super) fn layout_deployment(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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
pub(super) fn layout_deploy_node(
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

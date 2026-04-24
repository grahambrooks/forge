use super::deployment::DEPLOY_GAP;
use super::*;

const BOUNDARY_PAD: f64 = 24.0;
const BOUNDARY_HEADER: f64 = 34.0;
const BOUNDARY_MEMBER_W: f64 = 180.0;
const BOUNDARY_MEMBER_H: f64 = 60.0;
const BOUNDARY_MEMBER_GAP: f64 = 16.0;

pub(super) fn layout_trust_boundary(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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

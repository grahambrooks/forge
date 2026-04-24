use super::deployment::DEPLOY_GAP;
use super::*;

const TEAM_W: f64 = 500.0;
const TEAM_HEADER_H: f64 = 36.0;
const TEAM_MEMBER_H: f64 = 28.0;
const TEAM_PAD: f64 = 14.0;

pub(super) fn layout_team_map(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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

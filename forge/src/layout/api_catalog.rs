use super::deployment::DEPLOY_GAP;
use super::*;

pub(super) fn layout_api_catalog(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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

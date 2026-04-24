use super::*;

// ─── Tech Stack View ─────────────────────────────────────────────

const TECH_CARD_W: f64 = 180.0;
const TECH_CARD_H: f64 = 60.0;
const TECH_GAP: f64 = 14.0;
const TECH_CAT_PAD: f64 = 16.0;
const TECH_CAT_HEADER: f64 = 32.0;
const TECH_COLS: usize = 4;

pub(super) fn layout_tech_stack(model: &Model, view: &View, _tm: &TextMeasurer) -> Layout {
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

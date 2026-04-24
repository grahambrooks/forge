use super::*;

pub(super) fn layout_pipeline(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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

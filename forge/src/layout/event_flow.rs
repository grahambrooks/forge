use super::*;

pub(super) fn layout_event_flow(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    // Pick a topic-box width that fits the widest topic name OR
    // description across all flows, plus padding. Otherwise long
    // descriptions overflow the box border.
    let topic_w = model
        .event_flows
        .iter()
        .map(|flow| {
            let name = flow.topic.as_deref().unwrap_or(&flow.name);
            let name_w = tm.measure(name, &FONT_NAME);
            let desc_w = flow
                .description
                .as_deref()
                .map(|d| tm.measure(&format!("[{}]", d), &FONT_TECH))
                .unwrap_or(0.0);
            name_w.max(desc_w)
        })
        .fold(200.0_f64, f64::max)
        + 30.0;
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

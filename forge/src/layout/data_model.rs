use super::*;

const ENTITY_W: f64 = 220.0;
const ENTITY_HEADER_H: f64 = 32.0;
const ENTITY_FIELD_H: f64 = 20.0;
const ENTITY_PAD: f64 = 10.0;
const ENTITY_GAP: f64 = 60.0;
const ENTITY_COLS: usize = 3;

pub(super) fn layout_data_model(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
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
        // Leave 60px above the entity row for the U-routed relationship
        // lines and their label pills (see render_edge / render_edge_label).
        let y = TITLE_H + 60.0 + row as f64 * (200.0 + ENTITY_GAP);

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

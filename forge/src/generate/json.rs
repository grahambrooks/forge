use super::util::{json_esc, kind_display};
use super::*;
pub(super) fn render_json(model: &Model) -> String {
    let mut o = String::from("{\n");
    o.push_str(&format!("  \"name\": \"{}\",\n", json_esc(&model.name)));
    o.push_str(&format!(
        "  \"description\": \"{}\",\n",
        json_esc(&model.description)
    ));

    // Elements
    o.push_str("  \"elements\": [\n");
    let els: Vec<&Element> = model.elements.values().collect();
    for (i, el) in els.iter().enumerate() {
        let comma = if i + 1 < els.len() { "," } else { "" };
        let tags = el
            .tags
            .iter()
            .map(|t| format!("\"{}\"", json_esc(t)))
            .collect::<Vec<_>>()
            .join(", ");
        o.push_str(&format!(
            "    {{\"id\": \"{}\", \"kind\": \"{}\", \"name\": \"{}\", \"description\": {}, \"technology\": {}, \"tags\": [{}], \"parent\": {}}}{}\n",
            json_esc(&el.id),
            kind_display(el.kind),
            json_esc(&el.name),
            el.description.as_ref().map(|d| format!("\"{}\"", json_esc(d))).unwrap_or_else(|| "null".into()),
            el.technology.as_ref().map(|t| format!("\"{}\"", json_esc(t))).unwrap_or_else(|| "null".into()),
            tags,
            el.parent.as_ref().map(|p| format!("\"{}\"", json_esc(p))).unwrap_or_else(|| "null".into()),
            comma
        ));
    }
    o.push_str("  ],\n");

    // Relationships
    o.push_str("  \"relationships\": [\n");
    for (i, r) in model.relationships.iter().enumerate() {
        let comma = if i + 1 < model.relationships.len() {
            ","
        } else {
            ""
        };
        o.push_str(&format!(
            "    {{\"from\": \"{}\", \"to\": \"{}\", \"label\": \"{}\", \"technology\": {}}}{}\n",
            json_esc(&r.frm),
            json_esc(&r.to),
            json_esc(&r.label),
            r.technology
                .as_ref()
                .map(|t| format!("\"{}\"", json_esc(t)))
                .unwrap_or_else(|| "null".into()),
            comma
        ));
    }
    o.push_str("  ]\n}\n");
    o
}

// ─── HTML Template ───────────────────────────────────────────────

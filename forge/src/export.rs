//! Forge export — serialize a Model to JSON or YAML.

use serde_json::json;

use crate::model::*;

pub fn to_json(model: &Model) -> String {
    let elements: Vec<serde_json::Value> = model
        .elements
        .values()
        .map(|el| {
            json!({
                "id": el.id,
                "kind": format!("{:?}", el.kind),
                "name": el.name,
                "description": el.description,
                "technology": el.technology,
                "tags": el.tags,
                "parent": el.parent,
                "properties": el.properties,
                "children": el.children,
            })
        })
        .collect();

    let relationships: Vec<serde_json::Value> = model
        .relationships
        .iter()
        .map(|r| {
            json!({
                "from": r.frm,
                "to": r.to,
                "label": r.label,
                "technology": r.technology,
            })
        })
        .collect();

    let views: Vec<serde_json::Value> = model
        .views
        .iter()
        .map(|v| {
            json!({
                "kind": format!("{:?}", v.kind),
                "key": v.key,
                "scope": v.scope,
                "title": v.title,
                "auto-layout": format!("{:?}", v.auto_layout),
                "animated": !v.animation.is_empty(),
            })
        })
        .collect();

    let doc = json!({
        "name": model.name,
        "description": model.description,
        "elements": elements,
        "relationships": relationships,
        "views": views,
        "tech-stack": model.tech_stack.iter().map(|c| json!({
            "category": c.name,
            "entries": c.entries.iter().map(|e| json!({
                "name": e.name, "version": e.version, "purpose": e.purpose,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "dataEntities": model.data_entities.iter().map(|e| json!({
            "name": e.name,
            "owner": e.owner,
            "fields": e.fields.iter().map(|f| json!({
                "name": f.name, "type": f.field_type, "constraints": f.constraints,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "trust-boundaries": model.trust_boundaries.iter().map(|b| json!({
            "name": b.name, "level": b.level, "members": b.members,
        })).collect::<Vec<_>>(),
        "teams": model.teams.iter().map(|t| json!({
            "name": t.name, "owns": t.owns, "contact": t.contact,
        })).collect::<Vec<_>>(),
    });

    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

pub fn to_yaml(model: &Model) -> String {
    // Convert via JSON value then to YAML
    let json_str = to_json(model);
    let value: serde_json::Value = serde_json::from_str(&json_str).unwrap_or_default();
    serde_yaml_ng::to_string(&value).unwrap_or(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn payments_model() -> Model {
        let text = include_str!("../examples/payments.forge");
        parser::parse(text).unwrap()
    }

    #[test]
    fn json_export_has_all_sections() {
        let json = to_json(&payments_model());
        assert!(json.contains("\"name\": \"Payment Platform\""));
        assert!(json.contains("\"elements\""));
        assert!(json.contains("\"relationships\""));
        assert!(json.contains("\"views\""));
        assert!(json.contains("\"tech-stack\""));
        assert!(json.contains("\"dataEntities\""));
        assert!(json.contains("\"trust-boundaries\""));
        assert!(json.contains("\"teams\""));
    }

    #[test]
    fn json_is_valid() {
        let json = to_json(&payments_model());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["elements"].as_array().unwrap().len() >= 20);
    }

    #[test]
    fn yaml_export_works() {
        let yaml = to_yaml(&payments_model());
        assert!(yaml.contains("Payment Platform"));
        assert!(yaml.contains("elements:"));
    }
}

//! Forge import — convert PlantUML C4 and Mermaid diagrams to .forge models.

use crate::analyze::emit;
use crate::model::*;

/// Detect format and import to a Model.
pub fn import(text: &str) -> Result<Model, String> {
    let trimmed = text.trim();
    if trimmed.starts_with("@startuml") || trimmed.contains("!include") && trimmed.contains("C4") {
        import_plantuml(text)
    } else if trimmed.starts_with("graph ")
        || trimmed.starts_with("flowchart ")
        || trimmed.starts_with("C4Context")
        || trimmed.starts_with("C4Container")
    {
        import_mermaid(text)
    } else {
        Err("Unrecognized format. Expected PlantUML C4 or Mermaid C4/flowchart.".into())
    }
}

/// Convert the imported model to .forge DSL text.
pub fn import_to_forge(text: &str) -> Result<String, String> {
    let model = import(text)?;
    Ok(emit::emit(&model))
}

// ─── PlantUML C4 Import ─────────────────────────────────────────

fn import_plantuml(text: &str) -> Result<Model, String> {
    let mut model = Model {
        name: "Imported".into(),
        ..Default::default()
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // Person(alias, "Name", "Description")
        if let Some(rest) = trimmed
            .strip_prefix("Person(")
            .or_else(|| trimmed.strip_prefix("Person_Ext("))
        {
            if let Some(el) = parse_c4_element(rest, ElementKind::Person) {
                model.add_element(el);
            }
        }
        // System(alias, "Name", "Description")
        else if let Some(rest) = trimmed
            .strip_prefix("System(")
            .or_else(|| trimmed.strip_prefix("System_Ext("))
            .or_else(|| trimmed.strip_prefix("System_Boundary("))
        {
            if let Some(el) = parse_c4_element(rest, ElementKind::System) {
                model.add_element(el);
            }
        }
        // Container(alias, "Name", "Technology", "Description")
        else if let Some(rest) = trimmed
            .strip_prefix("Container(")
            .or_else(|| trimmed.strip_prefix("ContainerDb("))
            .or_else(|| trimmed.strip_prefix("Container_Ext("))
        {
            if let Some(el) = parse_c4_container(rest) {
                model.add_element(el);
            }
        }
        // Component(alias, "Name", "Technology", "Description")
        else if let Some(rest) = trimmed.strip_prefix("Component(") {
            if let Some(el) = parse_c4_container_as(rest, ElementKind::Component) {
                model.add_element(el);
            }
        }
        // Rel(from, to, "label", "technology")
        else if let Some(rest) = trimmed
            .strip_prefix("Rel(")
            .or_else(|| trimmed.strip_prefix("Rel_D("))
            .or_else(|| trimmed.strip_prefix("Rel_U("))
            .or_else(|| trimmed.strip_prefix("Rel_L("))
            .or_else(|| trimmed.strip_prefix("Rel_R("))
            .or_else(|| trimmed.strip_prefix("BiRel("))
        {
            if let Some(rel) = parse_c4_rel(rest) {
                model.add_relationship(rel);
            }
        }
        // Title
        else if let Some(rest) = trimmed.strip_prefix("title ") {
            model.name = rest.trim().trim_matches('"').to_string();
        }
    }

    if model.elements.is_empty() {
        return Err("No elements found in PlantUML input".into());
    }
    Ok(model)
}

fn parse_c4_element(args: &str, kind: ElementKind) -> Option<Element> {
    let parts = split_c4_args(args);
    let id = parts.first()?.trim().to_string();
    let name = parts.get(1)?.trim().trim_matches('"').to_string();
    let mut el = Element::new(&id, kind, &name);
    if let Some(desc) = parts.get(2) {
        let d = desc.trim().trim_matches('"').to_string();
        if !d.is_empty() {
            el.description = Some(d);
        }
    }
    Some(el)
}

fn parse_c4_container(args: &str) -> Option<Element> {
    let parts = split_c4_args(args);
    let id = parts.first()?.trim().to_string();
    let name = parts.get(1)?.trim().trim_matches('"').to_string();
    let mut el = Element::new(&id, ElementKind::Container, &name);
    if let Some(tech) = parts.get(2) {
        let t = tech.trim().trim_matches('"').to_string();
        if !t.is_empty() {
            el.technology = Some(t);
        }
    }
    if let Some(desc) = parts.get(3) {
        let d = desc.trim().trim_matches('"').to_string();
        if !d.is_empty() {
            el.description = Some(d);
        }
    }
    Some(el)
}

fn parse_c4_container_as(args: &str, kind: ElementKind) -> Option<Element> {
    let mut el = parse_c4_container(args)?;
    el.kind = kind;
    Some(el)
}

fn parse_c4_rel(args: &str) -> Option<Relationship> {
    let parts = split_c4_args(args);
    let frm = parts.first()?.trim().to_string();
    let to = parts.get(1)?.trim().to_string();
    let label = parts
        .get(2)
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_default();
    let technology = parts
        .get(3)
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty());
    Some(Relationship {
        frm,
        to,
        label,
        technology,
        order: None,
    })
}

/// Split C4 function args: "a, \"b\", \"c\")" → ["a", "\"b\"", "\"c\""]
fn split_c4_args(args: &str) -> Vec<String> {
    let args = args.trim_end_matches(')').trim();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in args.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            current.push(ch);
        } else if ch == ',' && !in_quote {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

// ─── Mermaid Import ──────────────────────────────────────────────

fn import_mermaid(text: &str) -> Result<Model, String> {
    let mut model = Model {
        name: "Imported".into(),
        ..Default::default()
    };

    for line in text.lines() {
        let trimmed = line.trim();

        // C4Context / C4Container macros
        if let Some(rest) = trimmed
            .strip_prefix("Person(")
            .or_else(|| trimmed.strip_prefix("Person_Ext("))
        {
            if let Some(el) = parse_c4_element(rest, ElementKind::Person) {
                model.add_element(el);
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("System(")
            .or_else(|| trimmed.strip_prefix("System_Ext("))
        {
            if let Some(el) = parse_c4_element(rest, ElementKind::System) {
                model.add_element(el);
            }
        } else if let Some(rest) = trimmed.strip_prefix("Container(") {
            if let Some(el) = parse_c4_container(rest) {
                model.add_element(el);
            }
        }
        // Mermaid Rel
        else if let Some(rest) = trimmed
            .strip_prefix("Rel(")
            .or_else(|| trimmed.strip_prefix("BiRel("))
        {
            if let Some(rel) = parse_c4_rel(rest) {
                model.add_relationship(rel);
            }
        }
        // Mermaid flowchart: A[Label] --> B[Label]
        else if trimmed.contains("-->") && !trimmed.starts_with("%%") {
            if let Some((left, right)) = trimmed.split_once("-->") {
                let (from_id, from_label) = parse_mermaid_node(left.trim());
                let (to_id, to_label) = parse_mermaid_node(right.trim());
                // Extract edge label: A -->|label| B
                let edge_label = extract_mermaid_edge_label(trimmed);

                if !model.elements.contains_key(&from_id) {
                    let mut el = Element::new(&from_id, ElementKind::Container, &from_label);
                    el.tags.push("inferred".into());
                    model.add_element(el);
                }
                if !model.elements.contains_key(&to_id) {
                    let mut el = Element::new(&to_id, ElementKind::Container, &to_label);
                    el.tags.push("inferred".into());
                    model.add_element(el);
                }
                model.add_relationship(Relationship {
                    frm: from_id,
                    to: to_id,
                    label: edge_label,
                    technology: None,
                    order: None,
                });
            }
        }
        // Title
        else if let Some(rest) = trimmed.strip_prefix("title ") {
            model.name = rest.trim().to_string();
        }
    }

    if model.elements.is_empty() {
        return Err("No elements found in Mermaid input".into());
    }
    Ok(model)
}

fn parse_mermaid_node(s: &str) -> (String, String) {
    // A[Label] or A(Label) or A{Label} or just A
    let s = s.trim().trim_end_matches('|');
    if let Some(bracket) = s.find('[') {
        let id = s[..bracket].trim().to_string();
        let label = s[bracket + 1..].trim_end_matches(']').trim().to_string();
        (id, label)
    } else if let Some(paren) = s.find('(') {
        let id = s[..paren].trim().to_string();
        let label = s[paren + 1..].trim_end_matches(')').trim().to_string();
        (id, label)
    } else {
        let id = s.trim().to_string();
        (id.clone(), id)
    }
}

fn extract_mermaid_edge_label(line: &str) -> String {
    // A -->|label| B
    if let Some(start) = line.find("|") {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find('|') {
            return rest[..end].trim().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_plantuml_c4() {
        let puml = r#"
@startuml
!include C4_Context.puml
title Payment System

Person(customer, "Customer", "A user")
System(payments, "Payment Service", "Processes payments")
Rel(customer, payments, "Uses", "HTTPS")
@enduml
"#;
        let model = import(puml).unwrap();
        assert_eq!(model.name, "Payment System");
        assert_eq!(model.elements.len(), 2);
        assert_eq!(model.relationships.len(), 1);
        let cust = model.elements.get("customer").unwrap();
        assert_eq!(cust.kind, ElementKind::Person);
        assert_eq!(cust.name, "Customer");
    }

    #[test]
    fn import_plantuml_containers() {
        let puml = r#"
@startuml
Container(api, "API", "Rust", "REST gateway")
Container(db, "Database", "PostgreSQL", "Stores data")
Rel(api, db, "reads/writes", "SQL")
@enduml
"#;
        let model = import(puml).unwrap();
        let api = model.elements.get("api").unwrap();
        assert_eq!(api.technology.as_deref(), Some("Rust"));
        assert_eq!(api.description.as_deref(), Some("REST gateway"));
    }

    #[test]
    fn import_mermaid_flowchart() {
        let mmd = r#"
flowchart LR
    A[Web App] --> B[API Server]
    B --> C[Database]
"#;
        let model = import(mmd).unwrap();
        assert_eq!(model.elements.len(), 3);
        assert_eq!(model.relationships.len(), 2);
        let a = model.elements.get("A").unwrap();
        assert_eq!(a.name, "Web App");
    }

    #[test]
    fn import_mermaid_c4() {
        let mmd = r#"
C4Context
    title My System
    Person(user, "User", "End user")
    System(sys, "My System", "Does things")
    Rel(user, sys, "Uses")
"#;
        let model = import(mmd).unwrap();
        assert_eq!(model.name, "My System");
        assert_eq!(model.elements.len(), 2);
    }

    #[test]
    fn import_to_forge_roundtrip() {
        let puml = r#"
@startuml
Person(user, "User", "A person")
System(sys, "System", "A system")
Rel(user, sys, "uses", "HTTPS")
@enduml
"#;
        let forge = import_to_forge(puml).unwrap();
        assert!(forge.contains("forge \"Imported\""));
        assert!(forge.contains("user = person \"User\""));
        assert!(forge.contains("sys = system \"System\""));
    }

    #[test]
    fn import_mermaid_edge_labels() {
        let mmd = r#"
flowchart LR
    A[Svc A] -->|calls| B[Svc B]
"#;
        let model = import(mmd).unwrap();
        assert_eq!(model.relationships[0].label, "calls");
    }
}

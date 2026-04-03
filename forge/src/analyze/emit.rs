//! Forge DSL emitter — converts a Model to .forge text.

use crate::model::*;

pub fn emit(model: &Model) -> String {
    let mut o = String::new();
    let name = if model.name.is_empty() {
        "Untitled"
    } else {
        &model.name
    };
    o.push_str(&format!("forge \"{}\" {{\n", escape(name)));

    if !model.description.is_empty() {
        o.push_str(&format!(
            "  description \"{}\"\n",
            escape(&model.description)
        ));
    }

    // ── Model section ──
    let structural: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| {
            matches!(
                e.kind,
                ElementKind::Person
                    | ElementKind::System
                    | ElementKind::Container
                    | ElementKind::Component
            )
        })
        .collect();

    let has_model = !structural.is_empty() || !model.relationships.is_empty();
    if has_model {
        o.push_str("\n  model {\n");

        // Group: top-level elements (no parent) with their children
        let top_level: Vec<&Element> = structural
            .iter()
            .filter(|e| e.parent.is_none())
            .copied()
            .collect();

        for el in &top_level {
            emit_element(&mut o, model, el, 4);
        }

        // Relationships between top-level elements
        let structural_ids: std::collections::HashSet<&str> =
            structural.iter().map(|e| e.id.as_str()).collect();
        for rel in &model.relationships {
            if structural_ids.contains(rel.frm.as_str()) || structural_ids.contains(rel.to.as_str())
            {
                emit_relationship(&mut o, rel, 4);
            }
        }

        o.push_str("  }\n");
    }

    // ── Process section ──
    let pipelines: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Pipeline)
        .collect();
    let repos: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Repository)
        .collect();

    if !pipelines.is_empty() || !repos.is_empty() {
        o.push_str("\n  process {\n");

        for repo in &repos {
            emit_repository(&mut o, repo, 4);
        }

        for pipeline in &pipelines {
            emit_pipeline(&mut o, model, pipeline, 4);
        }

        o.push_str("  }\n");
    }

    // ── Views section ──
    if !model.views.is_empty() {
        o.push_str("\n  views {\n");
        for view in &model.views {
            emit_view(&mut o, view, 4);
        }
        o.push_str("  }\n");
    }

    // ── Docs section ──
    if !model.docs.is_empty() {
        o.push_str("\n  docs {\n");
        for doc in &model.docs {
            o.push_str(&format!(
                "    doc \"{}\" \"{}\"\n",
                escape(&doc.title),
                escape(&doc.path)
            ));
        }
        o.push_str("  }\n");
    }

    o.push_str("}\n");
    o
}

fn emit_element(o: &mut String, model: &Model, el: &Element, indent: usize) {
    let pad = " ".repeat(indent);
    let local_id = el.id.split('.').next_back().unwrap_or(&el.id);
    let kind_str = match el.kind {
        ElementKind::Person => "person",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        _ => return,
    };

    o.push_str(&format!(
        "{}{} = {} \"{}\"",
        pad,
        local_id,
        kind_str,
        escape(&el.name)
    ));

    let has_body = el.description.is_some()
        || el.technology.is_some()
        || !el.tags.is_empty()
        || !el.children.is_empty();

    if has_body {
        o.push_str(" {\n");
        let inner = " ".repeat(indent + 2);

        if let Some(ref desc) = el.description {
            o.push_str(&format!("{}description \"{}\"\n", inner, escape(desc)));
        }
        if let Some(ref tech) = el.technology {
            o.push_str(&format!("{}technology \"{}\"\n", inner, escape(tech)));
        }
        let visible_tags: Vec<&str> = el
            .tags
            .iter()
            .filter(|t| t.as_str() != "inferred")
            .map(|t| t.as_str())
            .collect();
        if !visible_tags.is_empty() {
            let tags_str: Vec<String> = visible_tags.iter().map(|t| format!("\"{}\"", t)).collect();
            o.push_str(&format!("{}tags {}\n", inner, tags_str.join(" ")));
        }

        // Emit children
        for child_id in &el.children {
            if let Some(child) = model.elements.get(child_id) {
                o.push('\n');
                emit_element(o, model, child, indent + 2);
            }
        }

        // Emit relationships where both endpoints are this element or its children
        let child_set: std::collections::HashSet<&str> =
            el.children.iter().map(|s| s.as_str()).collect();
        for rel in &model.relationships {
            let frm_is_child = child_set.contains(rel.frm.as_str());
            let to_is_child = child_set.contains(rel.to.as_str());
            if frm_is_child && to_is_child {
                emit_relationship(o, rel, indent + 2);
            }
        }

        o.push_str(&format!("{}}}\n", pad));
    } else {
        o.push('\n');
    }
}

fn emit_relationship(o: &mut String, rel: &Relationship, indent: usize) {
    let pad = " ".repeat(indent);
    let frm_local = rel.frm.split('.').next_back().unwrap_or(&rel.frm);
    let to_local = rel.to.split('.').next_back().unwrap_or(&rel.to);
    o.push_str(&format!("{}{} -> {}", pad, frm_local, to_local));
    if !rel.label.is_empty() {
        o.push_str(&format!(" \"{}\"", escape(&rel.label)));
    }
    if let Some(ref tech) = rel.technology {
        o.push_str(&format!(" \"{}\"", escape(tech)));
    }
    o.push('\n');
}

fn emit_repository(o: &mut String, el: &Element, indent: usize) {
    let pad = " ".repeat(indent);
    let local_id = el.id.split('.').next_back().unwrap_or(&el.id);
    o.push_str(&format!(
        "{}{} = repository \"{}\"",
        pad,
        local_id,
        escape(&el.name)
    ));

    let has_props = !el.properties.is_empty();
    if has_props {
        o.push_str(" {\n");
        let inner = " ".repeat(indent + 2);
        if let Some(url) = el.properties.get("url") {
            o.push_str(&format!("{}url \"{}\"\n", inner, escape(url)));
        }
        o.push_str(&format!("{}}}\n", pad));
    } else {
        o.push('\n');
    }
}

fn emit_pipeline(o: &mut String, model: &Model, pipeline: &Element, indent: usize) {
    let pad = " ".repeat(indent);
    o.push_str(&format!(
        "{}pipeline \"{}\" {{\n",
        pad,
        escape(&pipeline.name)
    ));

    let inner = " ".repeat(indent + 2);

    // Emit stages
    let stages: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Stage && e.parent.as_deref() == Some(&pipeline.id))
        .collect();

    for stage in &stages {
        let local_id = stage.id.split('.').next_back().unwrap_or(&stage.id);
        o.push_str(&format!(
            "{}{} = stage \"{}\"",
            inner,
            local_id,
            escape(&stage.name)
        ));

        let needs: Vec<&StageLink> = model
            .stage_links
            .iter()
            .filter(|l| l.to == stage.id)
            .collect();
        let has_env = stage.properties.contains_key("environment");
        let gates: Vec<&Element> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Gate && e.parent.as_deref() == Some(&stage.id))
            .collect();

        if !needs.is_empty() || has_env || !gates.is_empty() {
            o.push_str(" {\n");
            let deep = " ".repeat(indent + 4);

            for need in &needs {
                let dep_local = need.frm.split('.').next_back().unwrap_or(&need.frm);
                o.push_str(&format!("{}needs {}\n", deep, dep_local));
            }
            if let Some(env) = stage.properties.get("environment") {
                o.push_str(&format!("{}environment {}\n", deep, env));
            }
            for gate in &gates {
                o.push_str(&format!("{}gate \"{}\"\n", deep, escape(&gate.name)));
            }

            o.push_str(&format!("{}}}\n", inner));
        } else {
            o.push('\n');
        }
    }

    o.push_str(&format!("{}}}\n", pad));
}

fn emit_view(o: &mut String, view: &View, indent: usize) {
    let pad = " ".repeat(indent);
    let kind_str = match view.kind {
        ViewKind::SystemContext => "systemContext",
        ViewKind::Container => "container",
        ViewKind::PipelineView => "pipelineView",
    };

    let scope = view.scope.as_deref().unwrap_or("");
    let scope_ref = scope.split('.').next_back().unwrap_or(scope);

    if view.kind == ViewKind::PipelineView {
        o.push_str(&format!(
            "{}{} \"{}\" \"{}\" {{\n",
            pad,
            kind_str,
            escape(scope),
            escape(&view.key)
        ));
    } else {
        o.push_str(&format!(
            "{}{} {} \"{}\" {{\n",
            pad, kind_str, scope_ref, &view.key
        ));
    }

    let inner = " ".repeat(indent + 2);
    if view.include_all {
        o.push_str(&format!("{}include *\n", inner));
    }
    let layout_str = match view.auto_layout {
        AutoLayout::LeftRight => "lr",
        AutoLayout::TopBottom => "tb",
    };
    o.push_str(&format!("{}autoLayout {}\n", inner, layout_str));
    if let Some(ref title) = view.title {
        o.push_str(&format!("{}title \"{}\"\n", inner, escape(title)));
    }

    o.push_str(&format!("{}}}\n", pad));
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn roundtrip_payments() {
        // Parse → emit → re-parse should produce same element count
        let original = include_str!("../../examples/payments.forge");
        let model1 = parser::parse(original).unwrap();
        let emitted = emit(&model1);
        let model2 = parser::parse(&emitted).unwrap();

        assert_eq!(model1.name, model2.name);
        // Element counts should match (some may differ slightly due to
        // emit simplifications, but structural elements should match)
        let structural1 = model1
            .elements
            .values()
            .filter(|e| {
                matches!(
                    e.kind,
                    ElementKind::Person
                        | ElementKind::System
                        | ElementKind::Container
                        | ElementKind::Component
                )
            })
            .count();
        let structural2 = model2
            .elements
            .values()
            .filter(|e| {
                matches!(
                    e.kind,
                    ElementKind::Person
                        | ElementKind::System
                        | ElementKind::Container
                        | ElementKind::Component
                )
            })
            .count();
        assert_eq!(structural1, structural2);
        assert_eq!(model1.views.len(), model2.views.len());
    }

    #[test]
    fn emit_minimal_model() {
        let mut model = Model::default();
        model.name = "Test".into();
        let mut el = Element::new("svc", ElementKind::Container, "My Service");
        el.technology = Some("Rust".into());
        el.description = Some("A service".into());
        model.add_element(el);

        let text = emit(&model);
        assert!(text.contains("forge \"Test\""));
        assert!(text.contains("svc = container \"My Service\""));
        assert!(text.contains("technology \"Rust\""));
        assert!(text.contains("description \"A service\""));
    }

    #[test]
    fn emit_pipeline_model() {
        let mut model = Model::default();
        model.name = "CI".into();
        model.add_element(Element::new("ci", ElementKind::Pipeline, "CI Pipeline"));
        let mut stage = Element::new("ci.build", ElementKind::Stage, "Build");
        stage.parent = Some("ci".into());
        model.add_element(stage);

        let text = emit(&model);
        assert!(text.contains("pipeline \"CI Pipeline\""));
        assert!(text.contains("build = stage \"Build\""));
    }
}

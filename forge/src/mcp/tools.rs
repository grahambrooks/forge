use std::path::PathBuf;

use serde_json::{json, Value};

use super::{element_json, element_kind_counts, load_model, McpServer};
use crate::analyze;
use crate::check;
use crate::layout;
use crate::model::*;
use crate::parser;
use crate::render;

impl McpServer {
    pub(super) fn tool_analyze(&self, args: &Value) -> String {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        if !path.exists() {
            return json!({"error": format!("path does not exist: {}", path.display())})
                .to_string();
        }

        let scanners = args
            .get("scanners")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![
                    "code".into(),
                    "semantic".into(),
                    "ci".into(),
                    "docker".into(),
                    "git".into(),
                    "k8s".into(),
                    "infra".into(),
                    "diagrams".into(),
                ]
            });

        let mut exclude: Vec<String> = Vec::new();
        if let Some(arr) = args.get("exclude").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    exclude.push(s.to_string());
                }
            }
        }

        let out_path = args.get("out").and_then(|v| v.as_str()).map(PathBuf::from);

        let mut config = analyze::AnalyzeConfig {
            paths: vec![path.clone()],
            scanners,
            out: out_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("forge.forge")),
            dry_run: out_path.is_none(),
            ..Default::default()
        };
        config.exclude.extend(exclude);

        let fresh = analyze::analyze(&config);

        let merge = args.get("merge").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut state = self.state.borrow_mut();
        if merge && !state.model.elements.is_empty() {
            analyze::merge::merge(&mut state.model, fresh);
        } else {
            state.model = fresh;
        }

        let mut write_status = json!(null);
        if let Some(out) = &out_path {
            let text = analyze::emit::emit(&state.model);
            match std::fs::write(out, text) {
                Ok(_) => {
                    state.source = Some(out.clone());
                    write_status = json!({"wrote": out.display().to_string()});
                }
                Err(e) => {
                    write_status = json!({"error": format!("write {}: {}", out.display(), e)});
                }
            }
        }

        let kinds = element_kind_counts(&state.model);
        serde_json::to_string_pretty(&json!({
            "analyzed": path.display().to_string(),
            "name": state.model.name,
            "elements": state.model.elements.len(),
            "relationships": state.model.relationships.len(),
            "views": state.model.views.len(),
            "kinds": kinds,
            "source": state.source.as_ref().map(|p| p.display().to_string()),
            "file": write_status,
        }))
        .unwrap_or_default()
    }

    pub(super) fn tool_reload(&self, args: &Value) -> String {
        let override_path = args
            .get("source")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let mut state = self.state.borrow_mut();
        let path = match override_path.or_else(|| state.source.clone()) {
            Some(p) => p,
            None => {
                return json!({
                    "error": "no source to reload. Pass `source`, or call forge_analyze with an `out` path first."
                })
                .to_string();
            }
        };

        match load_model(&path) {
            Ok(m) => {
                state.model = m;
                state.source = Some(path.clone());
                serde_json::to_string_pretty(&json!({
                    "reloaded": path.display().to_string(),
                    "elements": state.model.elements.len(),
                    "relationships": state.model.relationships.len(),
                    "views": state.model.views.len(),
                }))
                .unwrap_or_default()
            }
            Err(e) => json!({"error": e}).to_string(),
        }
    }

    pub(super) fn tool_overview(&self) -> String {
        let state = self.state.borrow();
        let model = &state.model;
        if model.elements.is_empty() && model.views.is_empty() {
            return json!({
                "empty": true,
                "hint": "No model loaded. Call forge_analyze to infer one from a repository, or pass --source at startup.",
            })
            .to_string();
        }

        let top_systems: Vec<Value> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::System && e.parent.is_none())
            .map(|e| json!({"id": e.id, "name": e.name}))
            .collect();

        let top_containers: Vec<Value> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Container && e.parent.is_none())
            .map(|e| json!({"id": e.id, "name": e.name, "technology": e.technology}))
            .collect();

        let views: Vec<Value> = model
            .views
            .iter()
            .map(|v| json!({"key": v.key, "kind": format!("{:?}", v.kind), "title": v.title}))
            .collect();

        serde_json::to_string_pretty(&json!({
            "name": model.name,
            "description": model.description,
            "source": state.source.as_ref().map(|p| p.display().to_string()),
            "counts": {
                "elements": model.elements.len(),
                "relationships": model.relationships.len(),
                "views": model.views.len(),
                "tech_categories": model.tech_stack.len(),
                "data_entities": model.data_entities.len(),
                "teams": model.teams.len(),
                "trust_boundaries": model.trust_boundaries.len(),
            },
            "by_kind": element_kind_counts(model),
            "top_level_systems": top_systems,
            "top_level_containers": top_containers,
            "views": views,
        }))
        .unwrap_or_default()
    }

    pub(super) fn tool_list_views(&self) -> String {
        let state = self.state.borrow();
        let views: Vec<Value> = state
            .model
            .views
            .iter()
            .map(|v| {
                json!({
                    "key": v.key,
                    "kind": format!("{:?}", v.kind),
                    "title": v.title,
                    "scope": v.scope,
                })
            })
            .collect();
        serde_json::to_string_pretty(&views).unwrap_or_default()
    }

    pub(super) fn tool_query(&self, args: &Value) -> String {
        let kind = args.get("kind").and_then(|v| v.as_str());
        let tag = args.get("tag").and_then(|v| v.as_str());
        let name = args.get("name").and_then(|v| v.as_str());

        let state = self.state.borrow();
        let results: Vec<Value> = state
            .model
            .elements
            .values()
            .filter(|el| {
                if let Some(k) = kind {
                    if format!("{:?}", el.kind).to_lowercase() != k.to_lowercase() {
                        return false;
                    }
                }
                if let Some(t) = tag {
                    if !el.tags.iter().any(|et| et.eq_ignore_ascii_case(t)) {
                        return false;
                    }
                }
                if let Some(n) = name {
                    if !el.name.to_lowercase().contains(&n.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .map(element_json)
            .collect();

        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    pub(super) fn tool_render(&self, args: &Value) -> String {
        let view_key = args.get("view_key").and_then(|v| v.as_str()).unwrap_or("");
        let style = args
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("outline");

        let state = self.state.borrow();
        let view = match state.model.views.iter().find(|v| v.key == view_key) {
            Some(v) => v.clone(),
            None => {
                let keys: Vec<&str> = state.model.views.iter().map(|v| v.key.as_str()).collect();
                return format!(
                    "View '{}' not found. Available: {}",
                    view_key,
                    keys.join(", ")
                );
            }
        };

        let lo = layout::compute_layout(&state.model, &view);
        render::render_svg(&lo, style)
    }

    pub(super) fn tool_check(&self, args: &Value) -> String {
        let severity_str = args.get("severity").and_then(|v| v.as_str());
        let min = severity_str
            .and_then(check::Severity::from_str)
            .unwrap_or(check::Severity::Warning);

        let state = self.state.borrow();
        let violations = check::check(&state.model, min);
        let results: Vec<Value> = violations
            .iter()
            .map(|v| {
                json!({
                    "severity": v.severity.to_string(),
                    "rule": v.rule,
                    "element": v.element_id,
                    "message": v.message,
                })
            })
            .collect();
        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    pub(super) fn tool_element_detail(&self, args: &Value) -> String {
        let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.state.borrow();
        let el = match state.model.elements.get(id) {
            Some(e) => e,
            None => return format!("Element '{}' not found", id),
        };

        let outgoing: Vec<Value> = state
            .model
            .relationships
            .iter()
            .filter(|r| r.frm == id)
            .map(|r| json!({"to": r.to, "label": r.label, "technology": r.technology}))
            .collect();
        let incoming: Vec<Value> = state
            .model
            .relationships
            .iter()
            .filter(|r| r.to == id)
            .map(|r| json!({"from": r.frm, "label": r.label, "technology": r.technology}))
            .collect();
        let children: Vec<Value> = state
            .model
            .elements
            .values()
            .filter(|e| e.parent.as_deref() == Some(id))
            .map(|e| json!({"id": e.id, "kind": format!("{:?}", e.kind), "name": e.name}))
            .collect();

        serde_json::to_string_pretty(&json!({
            "id": el.id, "kind": format!("{:?}", el.kind), "name": el.name,
            "description": el.description, "technology": el.technology,
            "tags": el.tags, "parent": el.parent, "properties": el.properties,
            "children": children,
            "outgoing_relationships": outgoing,
            "incoming_relationships": incoming,
        }))
        .unwrap_or_default()
    }

    pub(super) fn tool_search(&self, args: &Value) -> String {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let state = self.state.borrow();
        let mut results: Vec<(u32, Value)> = state
            .model
            .elements
            .values()
            .filter_map(|el| {
                let mut score = 0u32;
                if el.name.to_lowercase().contains(&query) {
                    score += 10;
                }
                if el.id.to_lowercase().contains(&query) {
                    score += 5;
                }
                if el
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query)
                {
                    score += 3;
                }
                if el
                    .technology
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query)
                {
                    score += 3;
                }
                if score > 0 {
                    Some((
                        score,
                        json!({"id": el.id, "name": el.name, "kind": format!("{:?}", el.kind), "score": score}),
                    ))
                } else {
                    None
                }
            })
            .collect();
        results.sort_by_key(|r| std::cmp::Reverse(r.0));
        let results: Vec<Value> = results.into_iter().map(|(_, v)| v).collect();
        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    pub(super) fn tool_validate(&self, args: &Value) -> String {
        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
        match parser::parse(code) {
            Ok(m) => serde_json::to_string_pretty(&json!({
                "valid": true, "name": m.name,
                "elements": m.elements.len(),
                "relationships": m.relationships.len(),
                "views": m.views.len(),
            }))
            .unwrap_or_default(),
            Err(e) => serde_json::to_string_pretty(&json!({
                "valid": false, "error": e.msg, "line": e.line, "col": e.col,
            }))
            .unwrap_or_default(),
        }
    }
}

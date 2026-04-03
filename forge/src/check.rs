//! Forge architectural linter — built-in rules for model validation.

use std::collections::{HashMap, HashSet};

use crate::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

impl Severity {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Severity::Info),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub element_id: Option<String>,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.element_id.as_deref().unwrap_or("-");
        write!(
            f,
            "[{}] {} ({}): {}",
            self.severity, self.rule, id, self.message
        )
    }
}

pub fn check(model: &Model, min_severity: Severity) -> Vec<Violation> {
    let mut violations = Vec::new();

    check_missing_descriptions(model, &mut violations);
    check_missing_technology(model, &mut violations);
    check_orphaned_elements(model, &mut violations);
    check_dependency_cycles(model, &mut violations);
    check_database_direct_access(model, &mut violations);
    check_chatty_coupling(model, &mut violations);
    check_gate_coverage(model, &mut violations);
    check_empty_views(model, &mut violations);

    violations.retain(|v| v.severity >= min_severity);
    violations.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.rule.cmp(b.rule)));
    violations
}

// ── missing-descriptions ─────────────────────────────────────────

fn check_missing_descriptions(model: &Model, violations: &mut Vec<Violation>) {
    for el in model.elements.values() {
        // Only check structural elements, not process internals
        if !matches!(
            el.kind,
            ElementKind::Person
                | ElementKind::System
                | ElementKind::Container
                | ElementKind::Component
        ) {
            continue;
        }
        if el.description.is_none() || el.description.as_deref() == Some("") {
            violations.push(Violation {
                rule: "missing-descriptions",
                severity: Severity::Warning,
                message: format!("{} '{}' has no description", kind_name(el.kind), el.name),
                element_id: Some(el.id.clone()),
            });
        }
    }
}

// ── missing-technology ───────────────────────────────────────────

fn check_missing_technology(model: &Model, violations: &mut Vec<Violation>) {
    for el in model.elements.values() {
        if el.kind != ElementKind::Container {
            continue;
        }
        if el.technology.is_none() || el.technology.as_deref() == Some("") {
            violations.push(Violation {
                rule: "missing-technology",
                severity: Severity::Warning,
                message: format!("Container '{}' has no technology tag", el.name),
                element_id: Some(el.id.clone()),
            });
        }
    }
}

// ── orphaned-elements ────────────────────────────────────────────

fn check_orphaned_elements(model: &Model, violations: &mut Vec<Violation>) {
    let involved: HashSet<&str> = model
        .relationships
        .iter()
        .flat_map(|r| [r.frm.as_str(), r.to.as_str()])
        .collect();

    // Also consider stage links
    let stage_involved: HashSet<&str> = model
        .stage_links
        .iter()
        .flat_map(|l| [l.frm.as_str(), l.to.as_str()])
        .collect();

    // And parent-child relationships
    let has_children: HashSet<&str> = model
        .elements
        .values()
        .filter(|e| !e.children.is_empty())
        .map(|e| e.id.as_str())
        .collect();

    let has_parent: HashSet<&str> = model
        .elements
        .values()
        .filter_map(|e| e.parent.as_deref())
        .collect();

    for el in model.elements.values() {
        // Skip process-internal elements (gates, steps, etc.)
        if matches!(
            el.kind,
            ElementKind::Gate | ElementKind::Step | ElementKind::Artifact
        ) {
            continue;
        }
        // Skip elements that are part of a pipeline (stages have parent links)
        if el.kind == ElementKind::Pipeline {
            continue;
        }

        let id = el.id.as_str();
        let connected = involved.contains(id)
            || stage_involved.contains(id)
            || has_children.contains(id)
            || has_parent.contains(id);

        if !connected {
            violations.push(Violation {
                rule: "orphaned-elements",
                severity: Severity::Info,
                message: format!(
                    "{} '{}' has no relationships (may indicate incomplete modeling)",
                    kind_name(el.kind),
                    el.name
                ),
                element_id: Some(el.id.clone()),
            });
        }
    }
}

// ── dependency-cycles ────────────────────────────────────────────

fn check_dependency_cycles(model: &Model, violations: &mut Vec<Violation>) {
    // Build adjacency list for containers and components
    let relevant: HashSet<&str> = model
        .elements
        .values()
        .filter(|e| matches!(e.kind, ElementKind::Container | ElementKind::Component))
        .map(|e| e.id.as_str())
        .collect();

    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for r in &model.relationships {
        if relevant.contains(r.frm.as_str()) && relevant.contains(r.to.as_str()) {
            adj.entry(r.frm.as_str()).or_default().push(r.to.as_str());
        }
    }

    // DFS cycle detection
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();
    let mut path = Vec::new();

    for &node in &relevant {
        if !visited.contains(node) {
            if let Some(cycle) = dfs_cycle(node, &adj, &mut visited, &mut on_stack, &mut path) {
                violations.push(Violation {
                    rule: "dependency-cycles",
                    severity: Severity::Error,
                    message: format!("Circular dependency detected: {}", cycle.join(" → ")),
                    element_id: Some(cycle[0].to_string()),
                });
            }
        }
    }
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>,
    on_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    visited.insert(node);
    on_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = adj.get(node) {
        for &next in neighbors {
            if !visited.contains(next) {
                if let Some(cycle) = dfs_cycle(next, adj, visited, on_stack, path) {
                    return Some(cycle);
                }
            } else if on_stack.contains(next) {
                // Found a cycle — extract the cycle path
                let start = path.iter().position(|&n| n == next).unwrap();
                let mut cycle: Vec<String> = path[start..].iter().map(|s| s.to_string()).collect();
                cycle.push(next.to_string());
                return Some(cycle);
            }
        }
    }

    path.pop();
    on_stack.remove(node);
    None
}

// ── database-direct-access ───────────────────────────────────────

fn check_database_direct_access(model: &Model, violations: &mut Vec<Violation>) {
    let db_ids: HashSet<&str> = model
        .elements
        .values()
        .filter(|e| e.tags.contains(&"database".to_string()))
        .map(|e| e.id.as_str())
        .collect();

    for r in &model.relationships {
        if !db_ids.contains(r.to.as_str()) {
            continue;
        }
        if let Some(source) = model.elements.get(&r.frm) {
            if matches!(source.kind, ElementKind::Person | ElementKind::System) {
                let db = model
                    .elements
                    .get(&r.to)
                    .map(|e| e.name.as_str())
                    .unwrap_or(&r.to);
                violations.push(Violation {
                    rule: "database-direct-access",
                    severity: Severity::Error,
                    message: format!(
                        "{} '{}' accesses database '{}' directly (should go through a service layer)",
                        kind_name(source.kind),
                        source.name,
                        db
                    ),
                    element_id: Some(r.frm.clone()),
                });
            }
        }
    }
}

// ── chatty-coupling ──────────────────────────────────────────────

fn check_chatty_coupling(model: &Model, violations: &mut Vec<Violation>) {
    let threshold = 3;
    let mut pair_count: HashMap<(&str, &str), usize> = HashMap::new();

    for r in &model.relationships {
        let key = if r.frm < r.to {
            (r.frm.as_str(), r.to.as_str())
        } else {
            (r.to.as_str(), r.frm.as_str())
        };
        *pair_count.entry(key).or_insert(0) += 1;
    }

    for ((a, b), count) in &pair_count {
        if *count > threshold {
            let a_name = model.elements.get(*a).map(|e| e.name.as_str()).unwrap_or(a);
            let b_name = model.elements.get(*b).map(|e| e.name.as_str()).unwrap_or(b);
            violations.push(Violation {
                rule: "chatty-coupling",
                severity: Severity::Warning,
                message: format!(
                    "'{}' and '{}' have {} relationships (threshold: {}); consider merging or introducing an API",
                    a_name, b_name, count, threshold
                ),
                element_id: Some(a.to_string()),
            });
        }
    }
}

// ── gate-coverage ────────────────────────────────────────────────

fn check_gate_coverage(model: &Model, violations: &mut Vec<Violation>) {
    for el in model.elements.values() {
        if el.kind != ElementKind::Stage {
            continue;
        }
        // Check if this stage deploys to production
        let is_prod = el
            .properties
            .get("environment")
            .map(|e| e.contains("prod"))
            .unwrap_or(false);
        if !is_prod {
            continue;
        }

        // Check if it has a gate
        let has_gate = model
            .elements
            .values()
            .any(|e| e.kind == ElementKind::Gate && e.parent.as_deref() == Some(&el.id));

        if !has_gate {
            violations.push(Violation {
                rule: "gate-coverage",
                severity: Severity::Error,
                message: format!(
                    "Stage '{}' deploys to production without a quality gate",
                    el.name
                ),
                element_id: Some(el.id.clone()),
            });
        }
    }
}

// ── empty-views ──────────────────────────────────────────────────

fn check_empty_views(model: &Model, violations: &mut Vec<Violation>) {
    for view in &model.views {
        let scope_id = view.scope.as_deref().unwrap_or("");
        let has_content = match view.kind {
            ViewKind::SystemContext => model.elements.contains_key(scope_id),
            ViewKind::Container => model
                .elements
                .values()
                .any(|e| e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Container),
            ViewKind::PipelineView => model
                .elements
                .values()
                .any(|e| e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Stage),
            ViewKind::Deployment => model.elements.values().any(|e| {
                e.kind == ElementKind::DeploymentNode
                    && e.properties.get("environment").map(|s| s.as_str()) == Some(scope_id)
            }),
            ViewKind::TechStack => !model.tech_stack.is_empty(),
            ViewKind::Branching => model.elements.values().any(|e| {
                e.kind == ElementKind::Branch
                    && e.properties.get("strategy").map(|s| s.as_str()) == Some(scope_id)
            }),
        };
        if !has_content {
            violations.push(Violation {
                rule: "empty-views",
                severity: Severity::Warning,
                message: format!("View '{}' has no visible elements", view.key),
                element_id: None,
            });
        }
    }
}

// ── Utilities ────────────────────────────────────────────────────

fn kind_name(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::System => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Repository => "Repository",
        ElementKind::Pipeline => "Pipeline",
        ElementKind::Stage => "Stage",
        ElementKind::Gate => "Gate",
        ElementKind::DeploymentNode => "Deployment Node",
        _ => "Element",
    }
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
    fn payments_has_no_errors() {
        let model = payments_model();
        let errors: Vec<_> = check(&model, Severity::Error)
            .into_iter()
            .filter(|v| v.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    }

    #[test]
    fn payments_missing_descriptions_on_databases() {
        // payments.forge: db and cache containers lack descriptions — linter should catch them
        let model = payments_model();
        let violations = check(&model, Severity::Warning);
        let missing_desc: Vec<_> = violations
            .iter()
            .filter(|v| v.rule == "missing-descriptions")
            .collect();
        assert_eq!(missing_desc.len(), 2);
        assert!(missing_desc
            .iter()
            .any(|v| v.element_id.as_deref() == Some("payments.db")));
        assert!(missing_desc
            .iter()
            .any(|v| v.element_id.as_deref() == Some("payments.cache")));
    }

    #[test]
    fn payments_no_missing_technology() {
        let model = payments_model();
        let violations = check(&model, Severity::Warning);
        let missing_tech: Vec<_> = violations
            .iter()
            .filter(|v| v.rule == "missing-technology")
            .collect();
        assert!(missing_tech.is_empty(), "unexpected: {:?}", missing_tech);
    }

    #[test]
    fn payments_no_dependency_cycles() {
        let model = payments_model();
        let violations = check(&model, Severity::Error);
        let cycles: Vec<_> = violations
            .iter()
            .filter(|v| v.rule == "dependency-cycles")
            .collect();
        assert!(cycles.is_empty(), "unexpected cycles: {:?}", cycles);
    }

    #[test]
    fn payments_has_gate_coverage() {
        let model = payments_model();
        let violations = check(&model, Severity::Error);
        let gate: Vec<_> = violations
            .iter()
            .filter(|v| v.rule == "gate-coverage")
            .collect();
        assert!(gate.is_empty(), "unexpected: {:?}", gate);
    }

    #[test]
    fn detects_missing_description() {
        let mut model = Model::default();
        model.add_element(Element::new("svc", ElementKind::Container, "My Service"));
        let violations = check(&model, Severity::Warning);
        assert!(violations
            .iter()
            .any(|v| v.rule == "missing-descriptions" && v.element_id.as_deref() == Some("svc")));
    }

    #[test]
    fn detects_missing_technology() {
        let mut model = Model::default();
        let mut el = Element::new("svc", ElementKind::Container, "My Service");
        el.description = Some("A service".into());
        model.add_element(el);
        let violations = check(&model, Severity::Warning);
        assert!(violations
            .iter()
            .any(|v| v.rule == "missing-technology" && v.element_id.as_deref() == Some("svc")));
    }

    #[test]
    fn detects_orphaned_element() {
        let mut model = Model::default();
        let mut el = Element::new("lonely", ElementKind::System, "Lonely System");
        el.description = Some("No friends".into());
        model.add_element(el);
        let violations = check(&model, Severity::Info);
        assert!(violations
            .iter()
            .any(|v| v.rule == "orphaned-elements" && v.element_id.as_deref() == Some("lonely")));
    }

    #[test]
    fn detects_dependency_cycle() {
        let mut model = Model::default();
        model.add_element(Element::new("a", ElementKind::Container, "A"));
        model.add_element(Element::new("b", ElementKind::Container, "B"));
        model.add_relationship(Relationship {
            frm: "a".into(),
            to: "b".into(),
            label: String::new(),
            technology: None,
        });
        model.add_relationship(Relationship {
            frm: "b".into(),
            to: "a".into(),
            label: String::new(),
            technology: None,
        });
        let violations = check(&model, Severity::Error);
        assert!(
            violations.iter().any(|v| v.rule == "dependency-cycles"),
            "expected cycle: {:?}",
            violations
        );
    }

    #[test]
    fn detects_database_direct_access() {
        let mut model = Model::default();
        model.add_element(Element::new("user", ElementKind::Person, "User"));
        let mut db = Element::new("db", ElementKind::Container, "DB");
        db.tags.push("database".into());
        model.add_element(db);
        model.add_relationship(Relationship {
            frm: "user".into(),
            to: "db".into(),
            label: "queries".into(),
            technology: None,
        });
        let violations = check(&model, Severity::Error);
        assert!(violations
            .iter()
            .any(|v| v.rule == "database-direct-access"));
    }

    #[test]
    fn detects_chatty_coupling() {
        let mut model = Model::default();
        model.add_element(Element::new("a", ElementKind::Container, "A"));
        model.add_element(Element::new("b", ElementKind::Container, "B"));
        for i in 0..4 {
            model.add_relationship(Relationship {
                frm: "a".into(),
                to: "b".into(),
                label: format!("rel{}", i),
                technology: None,
            });
        }
        let violations = check(&model, Severity::Warning);
        assert!(violations.iter().any(|v| v.rule == "chatty-coupling"));
    }

    #[test]
    fn detects_missing_gate_on_prod() {
        let mut model = Model::default();
        model.add_element(Element::new("pipe", ElementKind::Pipeline, "CI"));
        let mut stage = Element::new("pipe.deploy", ElementKind::Stage, "Deploy");
        stage.parent = Some("pipe".into());
        stage
            .properties
            .insert("environment".into(), "production".into());
        model.add_element(stage);
        let violations = check(&model, Severity::Error);
        assert!(violations.iter().any(|v| v.rule == "gate-coverage"));
    }

    #[test]
    fn detects_empty_view() {
        let mut model = Model::default();
        model.views.push(View {
            kind: ViewKind::Container,
            key: "Empty".into(),
            scope: Some("nonexistent".into()),
            title: None,
            auto_layout: AutoLayout::TopBottom,
            include_all: true,
        });
        let violations = check(&model, Severity::Warning);
        assert!(violations.iter().any(|v| v.rule == "empty-views"));
    }

    #[test]
    fn severity_filter_works() {
        let mut model = Model::default();
        // This will trigger orphaned-elements (info) and missing-descriptions (warning)
        model.add_element(Element::new("svc", ElementKind::System, "Svc"));
        let all = check(&model, Severity::Info);
        let warnings_up = check(&model, Severity::Warning);
        let errors_only = check(&model, Severity::Error);
        assert!(all.len() >= warnings_up.len());
        assert!(warnings_up.len() >= errors_only.len());
    }
}

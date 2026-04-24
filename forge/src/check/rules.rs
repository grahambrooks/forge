use super::{Severity, Violation};
use crate::model::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub(super) fn check_missing_descriptions(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_missing_technology(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_orphaned_elements(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_dependency_cycles(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_database_direct_access(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_chatty_coupling(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_gate_coverage(model: &Model, violations: &mut Vec<Violation>) {
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

pub(super) fn check_empty_views(model: &Model, violations: &mut Vec<Violation>) {
    for view in &model.views {
        let scope_id = view.scope.as_deref().unwrap_or("");
        let has_content =
            match view.kind {
                ViewKind::SystemContext => model.elements.contains_key(scope_id),
                ViewKind::Container => model.elements.values().any(|e| {
                    e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Container
                }),
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
                ViewKind::DataModel => !model.data_entities.is_empty(),
                ViewKind::TrustBoundaryView => !model.trust_boundaries.is_empty(),
                ViewKind::TeamMap => !model.teams.is_empty(),
                ViewKind::Component => model.elements.values().any(|e| {
                    e.parent.as_deref() == Some(scope_id) && e.kind == ElementKind::Component
                }),
                ViewKind::ApiCatalogView => !model.api_catalogs.is_empty(),
                ViewKind::EventFlowView => !model.event_flows.is_empty(),
                ViewKind::Dynamic => model
                    .elements
                    .values()
                    .any(|e| e.parent.as_deref() == Some(scope_id)),
                ViewKind::Composite => view
                    .composite
                    .as_ref()
                    .map(|c| !c.cells.is_empty())
                    .unwrap_or(false),
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

// ── data-class-boundary ──────────────────────────────────────────

/// Warn when a container carrying the `pii` data class is reachable from a
/// `person` without passing through a container tagged `gateway` or
/// `encryption`. Protected intermediates act as BFS stops — paths that
/// cross them are considered safe. The rule also fires when the first
/// unprotected target happens to be a component (not just a container),
/// so wrapping a PII store in a `pii-proxy` component you forgot to tag
/// still gets caught.
pub(super) fn check_data_class_boundary(model: &Model, violations: &mut Vec<Violation>) {
    let pii_targets: HashSet<&str> = model
        .elements
        .values()
        .filter(|e| e.data_classes.iter().any(|c| c.eq_ignore_ascii_case("pii")))
        .map(|e| e.id.as_str())
        .collect();
    if pii_targets.is_empty() {
        return;
    }

    // Forward adjacency for relationship graph traversal.
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for r in &model.relationships {
        adj.entry(r.frm.as_str()).or_default().push(r.to.as_str());
    }

    let is_protected = |id: &str| -> bool {
        model
            .elements
            .get(id)
            .map(|el| el.tags.iter().any(|t| t == "gateway" || t == "encryption"))
            .unwrap_or(false)
    };

    for person in model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Person)
    {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(person.id.as_str());
        visited.insert(person.id.as_str());

        while let Some(cur) = queue.pop_front() {
            let Some(nexts) = adj.get(cur) else { continue };
            for &nxt in nexts {
                if !visited.insert(nxt) {
                    continue;
                }
                if pii_targets.contains(nxt) {
                    let target_name = model
                        .elements
                        .get(nxt)
                        .map(|e| e.name.as_str())
                        .unwrap_or(nxt);
                    violations.push(Violation {
                        rule: "data-class-boundary",
                        severity: Severity::Warning,
                        message: format!(
                            "Person '{}' can reach PII-classified container '{}' without crossing a gateway or encryption boundary",
                            person.name, target_name
                        ),
                        element_id: Some(nxt.to_string()),
                    });
                    continue;
                }
                if is_protected(nxt) {
                    continue;
                }
                queue.push_back(nxt);
            }
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

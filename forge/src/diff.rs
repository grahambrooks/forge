//! Forge diff engine — compares two models and produces a typed changeset.
//!
//! Compares a baseline model against the current model and identifies:
//! - Added elements, relationships, views, and docs
//! - Removed elements, relationships, views, and docs
//! - Modified elements (changed description, technology, tags, etc.)
//! - Auto-generated human-readable change description

use std::collections::HashSet;

use crate::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct ElementChange {
    pub id: String,
    pub name: String,
    pub kind: ElementKind,
    pub change: ChangeKind,
    pub details: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RelationshipChange {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub change: ChangeKind,
}

#[derive(Debug, Clone)]
pub struct DocChange {
    pub title: String,
    pub change: ChangeKind,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub element_changes: Vec<ElementChange>,
    pub relationship_changes: Vec<RelationshipChange>,
    pub doc_changes: Vec<DocChange>,
    pub description: String,
    /// IDs of elements that were added or modified (for SVG highlighting)
    pub added_ids: HashSet<String>,
    pub modified_ids: HashSet<String>,
    pub removed_ids: HashSet<String>,
}

impl DiffResult {
    pub fn is_empty(&self) -> bool {
        self.element_changes.is_empty()
            && self.relationship_changes.is_empty()
            && self.doc_changes.is_empty()
    }

    pub fn added_count(&self) -> usize {
        self.element_changes
            .iter()
            .filter(|c| c.change == ChangeKind::Added)
            .count()
    }

    pub fn removed_count(&self) -> usize {
        self.element_changes
            .iter()
            .filter(|c| c.change == ChangeKind::Removed)
            .count()
    }

    pub fn modified_count(&self) -> usize {
        self.element_changes
            .iter()
            .filter(|c| c.change == ChangeKind::Modified)
            .count()
    }
}

/// Compare baseline model against current model and produce a DiffResult.
pub fn diff(baseline: &Model, current: &Model) -> DiffResult {
    let mut element_changes = Vec::new();
    let mut added_ids = HashSet::new();
    let mut modified_ids = HashSet::new();
    let mut removed_ids = HashSet::new();

    // Compare elements
    let baseline_ids: HashSet<&str> = baseline.elements.keys().map(|s| s.as_str()).collect();
    let current_ids: HashSet<&str> = current.elements.keys().map(|s| s.as_str()).collect();

    // Added elements
    for id in current_ids.difference(&baseline_ids) {
        let el = &current.elements[*id];
        // Skip deployment nodes and process-internal elements for cleaner output
        if matches!(
            el.kind,
            ElementKind::Gate | ElementKind::Step | ElementKind::Artifact
        ) {
            continue;
        }
        element_changes.push(ElementChange {
            id: el.id.clone(),
            name: el.name.clone(),
            kind: el.kind,
            change: ChangeKind::Added,
            details: vec![format!("New {} added", kind_name(el.kind))],
        });
        added_ids.insert(el.id.clone());
    }

    // Removed elements
    for id in baseline_ids.difference(&current_ids) {
        let el = &baseline.elements[*id];
        if matches!(
            el.kind,
            ElementKind::Gate | ElementKind::Step | ElementKind::Artifact
        ) {
            continue;
        }
        element_changes.push(ElementChange {
            id: el.id.clone(),
            name: el.name.clone(),
            kind: el.kind,
            change: ChangeKind::Removed,
            details: vec![format!("{} removed", kind_name(el.kind))],
        });
        removed_ids.insert(el.id.clone());
    }

    // Modified elements
    for id in current_ids.intersection(&baseline_ids) {
        let old = &baseline.elements[*id];
        let new = &current.elements[*id];
        let mut details = Vec::new();

        if old.name != new.name {
            details.push(format!("Renamed from '{}' to '{}'", old.name, new.name));
        }
        if old.description != new.description {
            if old.description.is_none() && new.description.is_some() {
                details.push("Description added".into());
            } else if old.description.is_some() && new.description.is_none() {
                details.push("Description removed".into());
            } else {
                details.push("Description changed".into());
            }
        }
        if old.technology != new.technology {
            details.push(format!(
                "Technology changed from '{}' to '{}'",
                old.technology.as_deref().unwrap_or("none"),
                new.technology.as_deref().unwrap_or("none")
            ));
        }
        if old.tags != new.tags {
            details.push("Tags changed".into());
        }
        if old.children.len() != new.children.len() {
            let added = new.children.len() as i64 - old.children.len() as i64;
            if added > 0 {
                details.push(format!("{} child element(s) added", added));
            } else {
                details.push(format!("{} child element(s) removed", -added));
            }
        }

        if !details.is_empty() {
            element_changes.push(ElementChange {
                id: new.id.clone(),
                name: new.name.clone(),
                kind: new.kind,
                change: ChangeKind::Modified,
                details,
            });
            modified_ids.insert(new.id.clone());
        }
    }

    // Compare relationships
    let relationship_changes = diff_relationships(baseline, current);

    // Compare docs
    let doc_changes = diff_docs(baseline, current);

    // Sort: added first, then modified, then removed
    element_changes.sort_by_key(|c| match c.change {
        ChangeKind::Added => 0,
        ChangeKind::Modified => 1,
        ChangeKind::Removed => 2,
    });

    // Auto-generate description
    let description = generate_description(
        &element_changes,
        &relationship_changes,
        &doc_changes,
        current,
        baseline,
    );

    DiffResult {
        element_changes,
        relationship_changes,
        doc_changes,
        description,
        added_ids,
        modified_ids,
        removed_ids,
    }
}

fn diff_relationships(baseline: &Model, current: &Model) -> Vec<RelationshipChange> {
    let mut changes = Vec::new();

    // Key relationships by (from, to, label)
    let baseline_set: HashSet<(&str, &str, &str)> = baseline
        .relationships
        .iter()
        .map(|r| (r.frm.as_str(), r.to.as_str(), r.label.as_str()))
        .collect();
    let current_set: HashSet<(&str, &str, &str)> = current
        .relationships
        .iter()
        .map(|r| (r.frm.as_str(), r.to.as_str(), r.label.as_str()))
        .collect();

    for &(frm, to, label) in current_set.difference(&baseline_set) {
        changes.push(RelationshipChange {
            frm: frm.into(),
            to: to.into(),
            label: label.into(),
            change: ChangeKind::Added,
        });
    }

    for &(frm, to, label) in baseline_set.difference(&current_set) {
        changes.push(RelationshipChange {
            frm: frm.into(),
            to: to.into(),
            label: label.into(),
            change: ChangeKind::Removed,
        });
    }

    changes
}

fn diff_docs(baseline: &Model, current: &Model) -> Vec<DocChange> {
    let mut changes = Vec::new();

    let baseline_titles: HashSet<&str> = baseline.docs.iter().map(|d| d.title.as_str()).collect();
    let current_titles: HashSet<&str> = current.docs.iter().map(|d| d.title.as_str()).collect();

    for title in current_titles.difference(&baseline_titles) {
        changes.push(DocChange {
            title: title.to_string(),
            change: ChangeKind::Added,
        });
    }

    for title in baseline_titles.difference(&current_titles) {
        changes.push(DocChange {
            title: title.to_string(),
            change: ChangeKind::Removed,
        });
    }

    changes
}

fn generate_description(
    element_changes: &[ElementChange],
    relationship_changes: &[RelationshipChange],
    doc_changes: &[DocChange],
    _current: &Model,
    _baseline: &Model,
) -> String {
    let added: Vec<&ElementChange> = element_changes
        .iter()
        .filter(|c| c.change == ChangeKind::Added)
        .collect();
    let removed: Vec<&ElementChange> = element_changes
        .iter()
        .filter(|c| c.change == ChangeKind::Removed)
        .collect();
    let modified: Vec<&ElementChange> = element_changes
        .iter()
        .filter(|c| c.change == ChangeKind::Modified)
        .collect();

    let rels_added = relationship_changes
        .iter()
        .filter(|r| r.change == ChangeKind::Added)
        .count();
    let rels_removed = relationship_changes
        .iter()
        .filter(|r| r.change == ChangeKind::Removed)
        .count();

    let mut parts = Vec::new();

    // Summarize structural changes
    if !added.is_empty() {
        let names: Vec<&str> = added.iter().map(|c| c.name.as_str()).collect();
        if names.len() <= 3 {
            parts.push(format!("Added {}", names.join(", ")));
        } else {
            parts.push(format!(
                "Added {} new elements including {}",
                names.len(),
                names[..2].join(", ")
            ));
        }
    }

    if !removed.is_empty() {
        let names: Vec<&str> = removed.iter().map(|c| c.name.as_str()).collect();
        if names.len() <= 3 {
            parts.push(format!("Removed {}", names.join(", ")));
        } else {
            parts.push(format!("Removed {} elements", names.len()));
        }
    }

    if !modified.is_empty() {
        let names: Vec<&str> = modified.iter().map(|c| c.name.as_str()).collect();
        if names.len() <= 3 {
            parts.push(format!("Modified {}", names.join(", ")));
        } else {
            parts.push(format!("Modified {} elements", names.len()));
        }
    }

    if rels_added > 0 {
        parts.push(format!(
            "{} new relationship{}",
            rels_added,
            if rels_added == 1 { "" } else { "s" }
        ));
    }
    if rels_removed > 0 {
        parts.push(format!(
            "{} relationship{} removed",
            rels_removed,
            if rels_removed == 1 { "" } else { "s" }
        ));
    }

    // Note new documentation
    let new_docs: Vec<&DocChange> = doc_changes
        .iter()
        .filter(|d| d.change == ChangeKind::Added)
        .collect();
    if !new_docs.is_empty() {
        let titles: Vec<&str> = new_docs.iter().map(|d| d.title.as_str()).collect();
        parts.push(format!("New documentation: {}", titles.join(", ")));
    }

    if parts.is_empty() {
        "No architectural changes detected.".into()
    } else {
        format!("{}.", parts.join(". "))
    }
}

fn kind_name(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "actor",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Pipeline => "pipeline",
        ElementKind::Stage => "stage",
        ElementKind::Repository => "repository",
        ElementKind::DeploymentNode => "deployment node",
        _ => "element",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(name: &str) -> Model {
        let mut m = Model::default();
        m.name = name.into();
        m
    }

    #[test]
    fn empty_models_no_diff() {
        let a = make_model("A");
        let b = make_model("B");
        let result = diff(&a, &b);
        assert!(result.is_empty());
    }

    #[test]
    fn detect_added_element() {
        let baseline = make_model("Base");
        let mut current = make_model("Current");
        current.add_element(Element::new("svc", ElementKind::Container, "My Service"));
        let result = diff(&baseline, &current);

        assert_eq!(result.added_count(), 1);
        assert!(result.added_ids.contains("svc"));
        assert!(result.description.contains("My Service"));
    }

    #[test]
    fn detect_removed_element() {
        let mut baseline = make_model("Base");
        baseline.add_element(Element::new("svc", ElementKind::Container, "Old Service"));
        let current = make_model("Current");
        let result = diff(&baseline, &current);

        assert_eq!(result.removed_count(), 1);
        assert!(result.removed_ids.contains("svc"));
        assert!(result.description.contains("Old Service"));
    }

    #[test]
    fn detect_modified_element() {
        let mut baseline = make_model("Base");
        let mut el = Element::new("svc", ElementKind::Container, "Service");
        el.technology = Some("Go".into());
        baseline.add_element(el);

        let mut current = make_model("Current");
        let mut el2 = Element::new("svc", ElementKind::Container, "Service");
        el2.technology = Some("Rust".into());
        current.add_element(el2);

        let result = diff(&baseline, &current);
        assert_eq!(result.modified_count(), 1);
        assert!(result.modified_ids.contains("svc"));

        let change = &result.element_changes[0];
        assert!(change.details[0].contains("Go"));
        assert!(change.details[0].contains("Rust"));
    }

    #[test]
    fn detect_added_relationship() {
        let baseline = make_model("Base");
        let mut current = make_model("Current");
        current.add_relationship(Relationship {
            frm: "a".into(),
            to: "b".into(),
            label: "calls".into(),
            technology: None,
        });

        let result = diff(&baseline, &current);
        assert_eq!(result.relationship_changes.len(), 1);
        assert_eq!(result.relationship_changes[0].change, ChangeKind::Added);
    }

    #[test]
    fn detect_new_doc() {
        let baseline = make_model("Base");
        let mut current = make_model("Current");
        current.docs.push(Doc {
            title: "New ADR".into(),
            path: "docs/adr.md".into(),
            order: 0,
        });

        let result = diff(&baseline, &current);
        assert_eq!(result.doc_changes.len(), 1);
        assert_eq!(result.doc_changes[0].change, ChangeKind::Added);
        assert!(result.description.contains("New ADR"));
    }

    #[test]
    fn description_summarizes_changes() {
        let mut baseline = make_model("Base");
        baseline.add_element(Element::new("old", ElementKind::Container, "Legacy"));

        let mut current = make_model("Current");
        current.add_element(Element::new("new1", ElementKind::Container, "Cache"));
        current.add_element(Element::new("new2", ElementKind::Container, "Queue"));

        let result = diff(&baseline, &current);
        assert!(result.description.contains("Cache"));
        assert!(result.description.contains("Queue"));
        assert!(result.description.contains("Added"));
        assert!(result.description.contains("Removed Legacy"));
    }

    #[test]
    fn unchanged_elements_not_in_diff() {
        let mut baseline = make_model("Base");
        baseline.add_element(Element::new("svc", ElementKind::Container, "Service"));

        let mut current = make_model("Current");
        current.add_element(Element::new("svc", ElementKind::Container, "Service"));

        let result = diff(&baseline, &current);
        assert!(result.is_empty());
    }
}

/// Forge semantic model — a typed directed graph of elements and relationships.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    // Structure domain
    Person,
    System,
    Container,
    Component,
    // Process domain
    Repository,
    Branch,
    Pipeline,
    Stage,
    Environment,
    Gate,
    Step,
    Artifact,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub kind: ElementKind,
    pub name: String,
    pub description: Option<String>,
    pub technology: Option<String>,
    pub tags: Vec<String>,
    pub parent: Option<String>,
    pub properties: HashMap<String, String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ViewKind {
    SystemContext,
    Container,
    PipelineView,
}

#[derive(Debug, Clone)]
pub enum AutoLayout {
    TopBottom,
    LeftRight,
}

#[derive(Debug, Clone)]
pub struct View {
    pub kind: ViewKind,
    pub scope: Option<String>, // element id the view is scoped to
    pub key: String,
    pub title: Option<String>,
    pub auto_layout: AutoLayout,
    pub include_all: bool,
}

#[derive(Debug, Clone)]
pub struct StageLink {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Default)]
pub struct Model {
    pub name: String,
    pub description: String,
    pub elements: HashMap<String, Element>,
    pub relationships: Vec<Relationship>,
    pub views: Vec<View>,
    pub stage_links: Vec<StageLink>, // "needs" edges between pipeline stages
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_element(&mut self, el: Element) {
        let id = el.id.clone();
        if let Some(ref parent_id) = el.parent {
            if let Some(parent) = self.elements.get_mut(parent_id) {
                parent.children.push(id.clone());
            }
        }
        self.elements.insert(id, el);
    }

    pub fn add_relationship(&mut self, rel: Relationship) {
        self.relationships.push(rel);
    }

    pub fn add_view(&mut self, view: View) {
        self.views.push(view);
    }

    /// Get all children of an element that match a given kind.
    pub fn children_of_kind(&self, parent_id: &str, kind: &ElementKind) -> Vec<&Element> {
        self.elements
            .values()
            .filter(|e| e.parent.as_deref() == Some(parent_id) && e.kind == *kind)
            .collect()
    }

    /// Get all relationships where `from` or `to` match any of the given ids.
    pub fn relationships_involving(&self, ids: &[&str]) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| ids.contains(&r.from.as_str()) || ids.contains(&r.to.as_str()))
            .collect()
    }

    /// Get relationships where both endpoints are in the given set.
    pub fn relationships_between(&self, ids: &[&str]) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| ids.contains(&r.from.as_str()) && ids.contains(&r.to.as_str()))
            .collect()
    }

    /// Resolve a dotted identifier like "payments.api" to a full element id.
    pub fn resolve_id(&self, dotted: &str) -> Option<String> {
        // Direct match first
        if self.elements.contains_key(dotted) {
            return Some(dotted.to_string());
        }
        // Try as a dotted child reference: "parent.child"
        if let Some((_parent, _child)) = dotted.split_once('.') {
            if self.elements.contains_key(dotted) {
                return Some(dotted.to_string());
            }
        }
        None
    }
}

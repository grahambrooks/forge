//! Forge semantic model — typed directed graph of elements and relationships.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementKind {
    Person,
    System,
    Container,
    Component,
    Repository,
    Branch,
    Pipeline,
    Stage,
    Environment,
    Gate,
    Step,
    Artifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    SystemContext,
    Container,
    PipelineView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLayout {
    TopBottom,
    LeftRight,
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

impl Element {
    pub fn new(id: impl Into<String>, kind: ElementKind, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            name: name.into(),
            description: None,
            technology: None,
            tags: Vec::new(),
            parent: None,
            properties: HashMap::new(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StageLink {
    pub frm: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct View {
    pub kind: ViewKind,
    pub key: String,
    pub scope: Option<String>,
    pub title: Option<String>,
    pub auto_layout: AutoLayout,
    pub include_all: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub name: String,
    pub description: String,
    pub elements: HashMap<String, Element>,
    pub relationships: Vec<Relationship>,
    pub views: Vec<View>,
    pub stage_links: Vec<StageLink>,
}

impl Model {
    pub fn add_element(&mut self, el: Element) {
        if let Some(ref parent_id) = el.parent {
            if let Some(parent) = self.elements.get_mut(parent_id) {
                if !parent.children.contains(&el.id) {
                    parent.children.push(el.id.clone());
                }
            }
        }
        self.elements.insert(el.id.clone(), el);
    }

    pub fn add_relationship(&mut self, rel: Relationship) {
        self.relationships.push(rel);
    }

    pub fn relationships_between(&self, ids: &HashSet<String>) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| ids.contains(&r.frm) && ids.contains(&r.to))
            .collect()
    }

    pub fn relationships_involving(&self, ids: &HashSet<String>) -> Vec<&Relationship> {
        self.relationships
            .iter()
            .filter(|r| ids.contains(&r.frm) || ids.contains(&r.to))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_element_sets_parent_children() {
        let mut model = Model::default();
        let parent = Element::new("sys", ElementKind::System, "System");
        model.add_element(parent);

        let mut child = Element::new("sys.api", ElementKind::Container, "API");
        child.parent = Some("sys".into());
        model.add_element(child);

        let parent = model.elements.get("sys").unwrap();
        assert!(parent.children.contains(&"sys.api".to_string()));
    }

    #[test]
    fn add_element_no_duplicate_children() {
        let mut model = Model::default();
        model.add_element(Element::new("sys", ElementKind::System, "System"));

        let mut child = Element::new("sys.api", ElementKind::Container, "API");
        child.parent = Some("sys".into());
        model.add_element(child.clone());
        model.add_element(child);

        let parent = model.elements.get("sys").unwrap();
        assert_eq!(parent.children.iter().filter(|c| c.as_str() == "sys.api").count(), 1);
    }

    #[test]
    fn relationships_between_filters_correctly() {
        let mut model = Model::default();
        model.add_relationship(Relationship { frm: "a".into(), to: "b".into(), label: "1".into(), technology: None });
        model.add_relationship(Relationship { frm: "a".into(), to: "c".into(), label: "2".into(), technology: None });
        model.add_relationship(Relationship { frm: "c".into(), to: "d".into(), label: "3".into(), technology: None });

        let ids: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let between = model.relationships_between(&ids);
        assert_eq!(between.len(), 2); // a->b and a->c
    }

    #[test]
    fn relationships_involving_includes_partial() {
        let mut model = Model::default();
        model.add_relationship(Relationship { frm: "a".into(), to: "b".into(), label: "1".into(), technology: None });
        model.add_relationship(Relationship { frm: "c".into(), to: "d".into(), label: "2".into(), technology: None });

        let ids: HashSet<String> = ["a"].iter().map(|s| s.to_string()).collect();
        let involving = model.relationships_involving(&ids);
        assert_eq!(involving.len(), 1);
        assert_eq!(involving[0].label, "1");
    }
}

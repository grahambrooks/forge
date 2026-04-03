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

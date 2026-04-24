//! Forge layout engine — assigns positions to elements for SVG rendering.
//!
//! The layout engine is split by view type: each `layout_X` function lives in
//! its own sub-module. This file holds the shared types, the constants used
//! across layouts, the element-measurement helpers, and the public entry
//! points.
//!
//! Dimensions match Structurizr-style proportions:
//!   - Structure elements: 240×120
//!   - Person elements: 240×160 (extra height for head+shoulders)
//!   - System elements: 280×140
//!   - Pipeline stages: 170×80
//!   - Gates: 70×70

use crate::model::*;
use crate::text::*;

mod api_catalog;
mod branching;
mod component;
mod container;
mod data_model;
mod deployment;
mod event_flow;
mod flow;
mod pipeline;
mod system_context;
mod team_map;
mod tech_stack;
mod trust;

#[cfg(test)]
mod tests;

use api_catalog::layout_api_catalog;
use branching::layout_branching;
use component::layout_component;
use container::layout_container;
use data_model::layout_data_model;
use deployment::layout_deployment;
use event_flow::layout_event_flow;
use flow::{layout_composite, layout_dynamic};
use pipeline::layout_pipeline;
use system_context::layout_system_context;
use team_map::layout_team_map;
use tech_stack::layout_tech_stack;
use trust::layout_trust_boundary;

// ── Dimensions ──

const NODE_W: f64 = 240.0;
const NODE_H: f64 = 120.0;
const PERSON_W: f64 = 240.0;
const PERSON_H: f64 = 160.0;
const SYSTEM_W: f64 = 280.0;
const SYSTEM_H: f64 = 140.0;
const STAGE_W: f64 = 170.0;
const STAGE_H: f64 = 80.0;
const GATE_W: f64 = 70.0;
const H_GAP: f64 = 80.0;
const V_GAP: f64 = 70.0;
const PAD: f64 = 60.0;
const TITLE_H: f64 = 50.0;

const BOX_PAD_X: f64 = 20.0;
const BOX_PAD_Y: f64 = 14.0;
const MIN_NODE_W: f64 = 160.0;
const MIN_NODE_H: f64 = 60.0;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LayoutNode {
    pub id: String,
    pub label: String,
    pub sublabel: Option<String>,
    pub kind: ElementKind,
    pub tags: Vec<String>,
    pub rect: Rect,
    pub description: Option<String>,
    pub depth: usize,
    pub children_ids: Vec<String>,
    pub data_classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub frm: String,
    pub to: String,
    pub label: String,
    pub technology: Option<String>,
    /// Step number for dynamic views. `None` for ordinary edges; `Some(n)`
    /// renders a circled step badge at the arrow midpoint.
    pub order: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
}

pub fn compute_layout(model: &Model, view: &View) -> Layout {
    let tm = TextMeasurer::new();
    compute_layout_with(model, view, &tm)
}

pub fn compute_layout_with(model: &Model, view: &View, tm: &TextMeasurer) -> Layout {
    match view.kind {
        ViewKind::SystemContext => layout_system_context(model, view, tm),
        ViewKind::Container => layout_container(model, view, tm),
        ViewKind::PipelineView => layout_pipeline(model, view, tm),
        ViewKind::Deployment => layout_deployment(model, view, tm),
        ViewKind::TechStack => layout_tech_stack(model, view, tm),
        ViewKind::Branching => layout_branching(model, view, tm),
        ViewKind::Component => layout_component(model, view, tm),
        ViewKind::DataModel => layout_data_model(model, view, tm),
        ViewKind::TrustBoundaryView => layout_trust_boundary(model, view, tm),
        ViewKind::TeamMap => layout_team_map(model, view, tm),
        ViewKind::ApiCatalogView => layout_api_catalog(model, view, tm),
        ViewKind::EventFlowView => layout_event_flow(model, view, tm),
        ViewKind::Dynamic => layout_dynamic(model, view, tm),
        ViewKind::Composite => layout_composite(model, view, tm),
    }
}

#[cfg(test)]
pub fn compute_layout_for_view(model: &Model, view_key: &str) -> Option<Layout> {
    model
        .views
        .iter()
        .find(|v| v.key == view_key)
        .map(|v| compute_layout(model, v))
}

// ─── Shared measurement helpers ──────────────────────────────────

/// Measure the required (width, height) for a structural element.
fn measure_element(el: &Element, tm: &TextMeasurer) -> (f64, f64) {
    let sub = el.technology.as_ref().map(|t| format!("[{}]", t));

    // Compute minimum width from the widest text line
    let name_w = tm.measure(&el.name, &FONT_NAME);
    let kind_label = kind_label_for(el.kind);
    let kind_w = kind_label
        .map(|k| tm.measure(&format!("[{}]", k), &FONT_KIND))
        .unwrap_or(0.0);
    let tech_w = sub
        .as_ref()
        .map(|s| tm.measure(s, &FONT_TECH))
        .unwrap_or(0.0);

    // For description, we'll wrap it — but need a target width first
    let base_w = name_w.max(kind_w).max(tech_w) + BOX_PAD_X * 2.0;
    // Clamp width: at least min, at most a reasonable max
    let target_w = base_w.clamp(MIN_NODE_W, 320.0);
    let desc_max_w = target_w - BOX_PAD_X * 2.0;

    let mut line_count = 1; // name
    if kind_label.is_some() {
        line_count += 1;
    }
    if let Some(ref desc) = el.description {
        let wrapped = tm.wrap(desc, desc_max_w, &FONT_DESC);
        line_count += wrapped.len().min(3); // max 3 lines of description
    }
    if sub.is_some() {
        line_count += 1;
    }

    let content_h = line_count as f64 * FONT_NAME.line_height;
    let h = (content_h + BOX_PAD_Y * 2.0).max(MIN_NODE_H);

    // For person elements, add head+shoulders space
    let (w, h) = match el.kind {
        ElementKind::Person => (target_w.max(PERSON_W), h + 62.0), // silhouette
        ElementKind::System => (target_w.max(SYSTEM_W), h.max(SYSTEM_H)),
        _ => (target_w, h),
    };

    (w, h)
}

fn kind_label_for(kind: ElementKind) -> Option<&'static str> {
    match kind {
        ElementKind::Person => Some("Person"),
        ElementKind::System => Some("Software System"),
        ElementKind::Container => Some("Container"),
        ElementKind::Component => Some("Component"),
        _ => None,
    }
}

fn dims_for(el: &Element, tm: &TextMeasurer) -> (f64, f64) {
    measure_element(el, tm)
}

fn make_node(el: &Element, x: f64, y: f64, tm: &TextMeasurer) -> LayoutNode {
    let (w, h) = dims_for(el, tm);
    let sub = el.technology.as_ref().map(|t| format!("[{}]", t));
    LayoutNode {
        id: el.id.clone(),
        label: el.name.clone(),
        sublabel: sub,
        kind: el.kind,
        tags: el.tags.clone(),
        rect: Rect { x, y, w, h },
        description: el.description.clone(),
        depth: 0,
        children_ids: Vec::new(),
        data_classes: el.data_classes.clone(),
    }
}

//! Emits a default set of views keyed off the element kinds actually present
//! in the model. Hand-authored `views {}` blocks are never overwritten.

use crate::model::*;

/// Emit default views keyed off the element kinds actually present in the
/// model. If the model already declares views we leave them alone.
pub(super) fn synthesize_views(model: &mut Model) {
    if !model.views.is_empty() {
        return;
    }

    let system_id = first_id(model, ElementKind::System);
    let container_id = first_id(model, ElementKind::Container);
    let component_id = first_id(model, ElementKind::Component);
    let pipeline_id = first_id(model, ElementKind::Pipeline);
    let deployment_id = first_id(model, ElementKind::DeploymentNode);
    let strategy_id = first_strategy_id(model);

    let mut views: Vec<View> = Vec::new();
    let context_scope = system_id.clone().or_else(|| container_id.clone());

    if let Some(scope) = context_scope.clone() {
        views.push(make_view(
            ViewKind::SystemContext,
            "SystemContext",
            Some(scope),
            "System Context",
            AutoLayout::LeftRight,
        ));
    }
    if let Some(scope) = system_id.clone().or_else(|| container_id.clone()) {
        views.push(make_view(
            ViewKind::Container,
            "Containers",
            Some(scope),
            "Containers",
            AutoLayout::TopBottom,
        ));
    }
    if let (Some(_), Some(c)) = (component_id.as_ref(), container_id.as_ref()) {
        views.push(make_view(
            ViewKind::Component,
            "Components",
            Some(c.clone()),
            "Components",
            AutoLayout::TopBottom,
        ));
    }
    if let Some(p) = pipeline_id {
        views.push(make_view(
            ViewKind::PipelineView,
            "Pipeline",
            Some(p),
            "CI/CD Pipeline",
            AutoLayout::LeftRight,
        ));
    }
    if let Some(d) = deployment_id {
        views.push(make_view(
            ViewKind::Deployment,
            "Deployment",
            Some(d),
            "Deployment",
            AutoLayout::TopBottom,
        ));
    }
    if let Some(s) = strategy_id {
        views.push(make_view(
            ViewKind::Branching,
            "Branching",
            Some(s),
            "Branching Strategy",
            AutoLayout::TopBottom,
        ));
    }
    if !model.tech_stack.is_empty() {
        views.push(make_view(
            ViewKind::TechStack,
            "TechStack",
            None,
            "Technology Stack",
            AutoLayout::TopBottom,
        ));
    }
    if !model.teams.is_empty() {
        views.push(make_view(
            ViewKind::TeamMap,
            "Teams",
            None,
            "Team Ownership",
            AutoLayout::TopBottom,
        ));
    }
    if !model.trust_boundaries.is_empty() {
        views.push(make_view(
            ViewKind::TrustBoundaryView,
            "TrustBoundaries",
            None,
            "Trust Boundaries",
            AutoLayout::TopBottom,
        ));
    }
    if !model.data_entities.is_empty() {
        views.push(make_view(
            ViewKind::DataModel,
            "DataModel",
            None,
            "Data Model",
            AutoLayout::TopBottom,
        ));
    }
    if !model.api_catalogs.is_empty() {
        views.push(make_view(
            ViewKind::ApiCatalogView,
            "APICatalog",
            None,
            "API Catalog",
            AutoLayout::TopBottom,
        ));
    }

    model.views = views;
}

fn first_id(model: &Model, kind: ElementKind) -> Option<String> {
    // Stable: keep lexical-id order so repeated runs produce the same view scopes.
    let mut ids: Vec<&str> = model
        .elements
        .values()
        .filter(|e| e.kind == kind)
        .map(|e| e.id.as_str())
        .collect();
    ids.sort();
    ids.first().map(|s| s.to_string())
}

/// Extract the strategy ID from branch elements. Branches carry a `strategy`
/// property pointing to their parent strategy (e.g. "github-flow", "trunk-based").
/// Returns the first strategy ID found, in stable lexical order.
fn first_strategy_id(model: &Model) -> Option<String> {
    let mut strategies: Vec<&str> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Branch)
        .filter_map(|e| e.properties.get("strategy"))
        .map(|s| s.as_str())
        .collect();
    strategies.sort();
    strategies.dedup();
    strategies.first().map(|s| s.to_string())
}

fn make_view(
    kind: ViewKind,
    key: &str,
    scope: Option<String>,
    title: &str,
    layout: AutoLayout,
) -> View {
    View {
        kind,
        key: key.into(),
        scope,
        title: Some(title.into()),
        auto_layout: layout,
        include_all: true,
        animation: Animation::default(),
        composite: None,
    }
}

use super::*;
use crate::parser;

fn payments_layout(view_key: &str) -> Layout {
    let text = include_str!("../../examples/payments.forge");
    let model = parser::parse(text).unwrap();
    compute_layout_for_view(&model, view_key).expect("view should exist")
}

#[test]
fn system_context_layout() {
    let lo = payments_layout("SystemContext");
    assert_eq!(
        lo.title.as_deref(),
        Some("Payment Platform — System Context")
    );
    // 1 actor (Customer) + 1 system (Payment Service)
    assert_eq!(lo.nodes.len(), 2);
    // collapsed relationships (customer->payments + internal self-edge)
    assert!(lo.edges.len() >= 1);
    assert!(lo.width > 0.0);
    assert!(lo.height > 0.0);
}

#[test]
fn system_context_node_kinds() {
    let lo = payments_layout("SystemContext");
    assert!(lo.nodes.iter().any(|n| n.kind == ElementKind::Person));
    assert!(lo.nodes.iter().any(|n| n.kind == ElementKind::System));
}

#[test]
fn container_layout() {
    let lo = payments_layout("Containers");
    assert_eq!(lo.title.as_deref(), Some("Payment Platform — Containers"));
    // 1 external (Customer) + 5 containers
    assert_eq!(lo.nodes.len(), 6);
    // edges between visible nodes
    assert!(lo.edges.len() >= 4);
}

#[test]
fn container_layout_has_database() {
    let lo = payments_layout("Containers");
    assert!(lo
        .nodes
        .iter()
        .any(|n| n.tags.contains(&"database".to_string())));
}

#[test]
fn pipeline_layout() {
    let lo = payments_layout("Pipeline");
    assert_eq!(lo.title.as_deref(), Some("Payment API — CI/CD Pipeline"));
    // 4 stages + 3 gates = 7 nodes
    let stage_count = lo
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::Stage)
        .count();
    let gate_count = lo
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::Gate)
        .count();
    assert_eq!(stage_count, 4);
    assert_eq!(gate_count, 3);
    // 3 edges between consecutive stages
    assert_eq!(lo.edges.len(), 3);
}

#[test]
fn pipeline_topological_order() {
    let lo = payments_layout("Pipeline");
    let stages: Vec<&LayoutNode> = lo
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::Stage)
        .collect();
    // Build should come before Security, Security before Staging, Staging before Prod
    let pos = |label: &str| stages.iter().position(|n| n.label.contains(label)).unwrap();
    assert!(pos("Build") < pos("Security"));
    assert!(pos("Security") < pos("Staging"));
    assert!(pos("Staging") < pos("Production"));
}

#[test]
fn nodes_have_positive_dimensions() {
    let lo = payments_layout("Containers");
    for n in &lo.nodes {
        assert!(n.rect.w > 0.0, "node {} has zero width", n.id);
        assert!(n.rect.h > 0.0, "node {} has zero height", n.id);
    }
}

#[test]
fn nodes_dont_overlap_in_pipeline() {
    let lo = payments_layout("Pipeline");
    let stages: Vec<&LayoutNode> = lo
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::Stage)
        .collect();
    for i in 1..stages.len() {
        let prev_right = stages[i - 1].rect.x + stages[i - 1].rect.w;
        assert!(
            stages[i].rect.x > prev_right,
            "stages {} and {} overlap",
            stages[i - 1].id,
            stages[i].id
        );
    }
}

#[test]
fn deployment_layout() {
    let lo = payments_layout("Deployment");
    assert!(lo.title.as_deref().unwrap().contains("Deployment"));
    // Should have deployment nodes + container instances
    let deploy_nodes = lo
        .nodes
        .iter()
        .filter(|n| n.kind == ElementKind::DeploymentNode)
        .count();
    assert!(
        deploy_nodes >= 5,
        "expected >= 5 deployment nodes, got {}",
        deploy_nodes
    );
    // Should have container instances nested inside
    let instances = lo
        .nodes
        .iter()
        .filter(|n| n.kind != ElementKind::DeploymentNode && n.depth > 0)
        .count();
    assert!(
        instances >= 3,
        "expected >= 3 container instances, got {}",
        instances
    );
    assert!(lo.width > 0.0);
    assert!(lo.height > 0.0);
}

use crate::model::*;

use super::run;
use super::tech_stack::{
    classify_tech_layer, LAYER_APP, LAYER_INFRASTRUCTURE, LAYER_PERSISTENCE, LAYER_SERVICE,
};

fn container(id: &str, tech: &str) -> Element {
    let mut c = Element::new(id, ElementKind::Container, id);
    c.technology = Some(tech.into());
    c
}

#[test]
fn synthesizes_system_around_orphan_containers() {
    let mut model = Model::default();
    model.name = "example".into();
    model.add_element(container("api", "Rust / Axum"));
    model.add_element(container("db", "PostgreSQL"));

    run(&mut model);

    let systems: Vec<_> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::System)
        .collect();
    assert_eq!(systems.len(), 1);
    let sys = systems[0];
    assert_eq!(sys.children.len(), 2);
    assert!(model
        .elements
        .get("api")
        .unwrap()
        .parent
        .as_deref()
        .is_some());
}

#[test]
fn synthesizes_user_for_web_container() {
    let mut model = Model::default();
    model.add_element(container("api", "Python / Flask"));
    run(&mut model);

    assert!(model
        .elements
        .values()
        .any(|e| e.kind == ElementKind::Person && e.name == "User"));
    assert!(model
        .relationships
        .iter()
        .any(|r| r.label == "uses" && r.to == "api"));
}

#[test]
fn synthesizes_developer_for_pipeline() {
    let mut model = Model::default();
    let mut pipe = Element::new("ci", ElementKind::Pipeline, "CI");
    pipe.tags.push("inferred".into());
    model.add_element(pipe);

    run(&mut model);

    assert!(model
        .elements
        .values()
        .any(|e| e.kind == ElementKind::Person && e.name == "Developer"));
}

#[test]
fn emits_default_views_for_present_kinds() {
    let mut model = Model::default();
    model.name = "example".into();
    model.add_element(container("api", "Go / Gin"));
    let mut p = Element::new("ci", ElementKind::Pipeline, "CI");
    p.tags.push("inferred".into());
    model.add_element(p);

    run(&mut model);

    let kinds: Vec<ViewKind> = model.views.iter().map(|v| v.kind).collect();
    assert!(kinds.contains(&ViewKind::SystemContext));
    assert!(kinds.contains(&ViewKind::Container));
    assert!(kinds.contains(&ViewKind::PipelineView));
    assert!(kinds.contains(&ViewKind::TechStack));
}

#[test]
fn synthesis_is_idempotent() {
    let mut model = Model::default();
    model.name = "example".into();
    model.add_element(container("api", "Rust / Axum"));
    run(&mut model);
    let before = (
        model.elements.len(),
        model.views.len(),
        model.relationships.len(),
        model.tech_stack.len(),
    );
    run(&mut model);
    let after = (
        model.elements.len(),
        model.views.len(),
        model.relationships.len(),
        model.tech_stack.len(),
    );
    assert_eq!(before, after);
}

#[test]
fn does_not_wrap_when_system_already_exists() {
    let mut model = Model::default();
    model.add_element(Element::new("mySys", ElementKind::System, "mySys"));
    let mut c = container("api", "Rust / Axum");
    c.parent = Some("mySys".into());
    model.add_element(c);

    run(&mut model);

    let systems: Vec<_> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::System)
        .collect();
    assert_eq!(systems.len(), 1);
    assert_eq!(systems[0].id, "mySys");
}

#[test]
fn synthesises_default_branching_strategy_when_none_exists() {
    let mut model = Model::default();
    model.name = "example".into();
    model.add_element(container("api", "Rust / Axum"));

    run(&mut model);

    let branches: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Branch)
        .collect();
    assert_eq!(
        branches.len(),
        1,
        "expected exactly one synthesised trunk branch"
    );
    let trunk = branches[0];
    assert_eq!(trunk.name, "main");
    assert_eq!(
        trunk.properties.get("strategy").map(|s| s.as_str()),
        Some("github-flow")
    );
    assert!(trunk.tags.iter().any(|t| t == "trunk"));

    let has_branching_view = model.views.iter().any(|v| v.kind == ViewKind::Branching);
    assert!(
        has_branching_view,
        "branching view should be synthesised alongside default branch"
    );

    // Branching view must be scoped to the strategy ID, not the branch ID
    let branching_view = model
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Branching)
        .expect("branching view must exist");
    assert_eq!(
        branching_view.scope.as_deref(),
        Some("github-flow"),
        "branching view must be scoped to strategy ID, not branch ID"
    );
}

#[test]
fn preserves_existing_branching_strategy() {
    let mut model = Model::default();
    model.name = "example".into();
    model.add_element(container("api", "Rust / Axum"));

    let mut trunk = Element::new("trunk-based.trunk", ElementKind::Branch, "main");
    trunk.parent = Some("trunk-based".into());
    trunk
        .properties
        .insert("strategy".into(), "trunk-based".into());
    model.add_element(trunk);

    run(&mut model);

    let branches: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Branch)
        .collect();
    assert_eq!(branches.len(), 1);
    assert_eq!(
        branches[0].properties.get("strategy").map(|s| s.as_str()),
        Some("trunk-based"),
        "existing strategy must not be overwritten"
    );

    // Branching view must be scoped to the existing strategy ID
    let branching_view = model
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Branching)
        .expect("branching view must exist");
    assert_eq!(
        branching_view.scope.as_deref(),
        Some("trunk-based"),
        "branching view must be scoped to strategy ID"
    );
}

#[test]
fn aggregates_tech_stack_by_layer() {
    let mut model = Model::default();
    model.add_element(container("api", "Rust / Axum"));
    model.add_element(container("worker", "Rust / Tokio"));
    model.add_element(container("web", "TypeScript / Next.js"));
    model.add_element(container("db", "PostgreSQL"));

    run(&mut model);

    let categories: Vec<&str> = model.tech_stack.iter().map(|c| c.name.as_str()).collect();

    // Next.js is a frontend framework → App layer
    assert!(categories.contains(&"App"), "expected an App layer");
    // Axum and Tokio are backend frameworks → Service layer
    assert!(categories.contains(&"Service"), "expected a Service layer");
    // PostgreSQL is a database → Persistence layer
    assert!(
        categories.contains(&"Persistence"),
        "expected a Persistence layer"
    );

    // Verify specific tech entries land in the correct layer
    let app_layer = model.tech_stack.iter().find(|c| c.name == "App").unwrap();
    assert!(
        app_layer.entries.iter().any(|e| e.name == "Next.js"),
        "Next.js should be in App layer"
    );

    let service_layer = model
        .tech_stack
        .iter()
        .find(|c| c.name == "Service")
        .unwrap();
    assert!(
        service_layer.entries.iter().any(|e| e.name == "Axum"),
        "Axum should be in Service layer"
    );

    let persistence_layer = model
        .tech_stack
        .iter()
        .find(|c| c.name == "Persistence")
        .unwrap();
    assert!(
        persistence_layer
            .entries
            .iter()
            .any(|e| e.name == "PostgreSQL"),
        "PostgreSQL should be in Persistence layer"
    );
}

#[test]
fn classify_tech_layer_routes_correctly() {
    assert_eq!(classify_tech_layer("React"), LAYER_APP);
    assert_eq!(classify_tech_layer("Next.js"), LAYER_APP);
    assert_eq!(classify_tech_layer("Vue"), LAYER_APP);
    assert_eq!(classify_tech_layer("Axum"), LAYER_SERVICE);
    assert_eq!(classify_tech_layer("Flask"), LAYER_SERVICE);
    assert_eq!(classify_tech_layer("Gin"), LAYER_SERVICE);
    assert_eq!(classify_tech_layer("Rust"), LAYER_SERVICE);
    assert_eq!(classify_tech_layer("Go"), LAYER_SERVICE);
    assert_eq!(classify_tech_layer("PostgreSQL"), LAYER_PERSISTENCE);
    assert_eq!(classify_tech_layer("Redis"), LAYER_PERSISTENCE);
    assert_eq!(classify_tech_layer("MongoDB"), LAYER_PERSISTENCE);
    assert_eq!(classify_tech_layer("Apache Kafka"), LAYER_PERSISTENCE);
    assert_eq!(classify_tech_layer("Docker"), LAYER_INFRASTRUCTURE);
    assert_eq!(classify_tech_layer("Kubernetes"), LAYER_INFRASTRUCTURE);
    assert_eq!(classify_tech_layer("Terraform"), LAYER_INFRASTRUCTURE);
    assert_eq!(classify_tech_layer("Nginx"), LAYER_INFRASTRUCTURE);
}

#[test]
fn classify_tech_layer_no_false_positives() {
    // "MyReactiveService" must NOT match "React"
    assert_eq!(
        classify_tech_layer("MyReactiveService"),
        LAYER_SERVICE,
        "substring match on 'React' inside 'MyReactiveService' should not classify as App"
    );
    // "PostgreSQL 16" (versioned) should still resolve to Persistence
    assert_eq!(
        classify_tech_layer("PostgreSQL 16"),
        LAYER_PERSISTENCE,
        "versioned name 'PostgreSQL 16' should be Persistence"
    );
    // "Redis 7" should be Persistence
    assert_eq!(classify_tech_layer("Redis 7"), LAYER_PERSISTENCE);
}

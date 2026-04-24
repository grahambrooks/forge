use super::*;
use crate::parser;

fn payments_model() -> Model {
    let text = include_str!("../../examples/payments.forge");
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
        order: None,
    });
    model.add_relationship(Relationship {
        frm: "b".into(),
        to: "a".into(),
        label: String::new(),
        technology: None,
        order: None,
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
        order: None,
    });
    let violations = check(&model, Severity::Error);
    assert!(violations
        .iter()
        .any(|v| v.rule == "database-direct-access"));
}

#[test]
fn detects_data_class_boundary_unprotected() {
    // customer -> api -> db where db has pii and api has no
    // gateway/encryption tag. Should fire.
    let mut model = Model::default();
    model.add_element(Element::new("customer", ElementKind::Person, "Customer"));
    model.add_element(Element::new("api", ElementKind::Container, "API"));
    let mut db = Element::new("db", ElementKind::Container, "Ledger DB");
    db.data_classes.push("pii".into());
    model.add_element(db);
    model.add_relationship(Relationship {
        frm: "customer".into(),
        to: "api".into(),
        label: "uses".into(),
        technology: None,
        order: None,
    });
    model.add_relationship(Relationship {
        frm: "api".into(),
        to: "db".into(),
        label: "reads".into(),
        technology: None,
        order: None,
    });
    let violations = check(&model, Severity::Warning);
    assert!(
        violations.iter().any(|v| v.rule == "data-class-boundary"),
        "expected data-class-boundary violation, got {:?}",
        violations.iter().map(|v| v.rule).collect::<Vec<_>>()
    );
}

#[test]
fn data_class_boundary_respects_gateway() {
    // Same as above but api is tagged `gateway`. The rule should not
    // fire because the protected intermediate blocks the BFS.
    let mut model = Model::default();
    model.add_element(Element::new("customer", ElementKind::Person, "Customer"));
    let mut api = Element::new("api", ElementKind::Container, "API");
    api.tags.push("gateway".into());
    model.add_element(api);
    let mut db = Element::new("db", ElementKind::Container, "Ledger DB");
    db.data_classes.push("pii".into());
    model.add_element(db);
    model.add_relationship(Relationship {
        frm: "customer".into(),
        to: "api".into(),
        label: "uses".into(),
        technology: None,
        order: None,
    });
    model.add_relationship(Relationship {
        frm: "api".into(),
        to: "db".into(),
        label: "reads".into(),
        technology: None,
        order: None,
    });
    let violations = check(&model, Severity::Warning);
    assert!(
        !violations.iter().any(|v| v.rule == "data-class-boundary"),
        "gateway intermediate should suppress data-class-boundary"
    );
}

#[test]
fn data_class_boundary_direct_access_fires() {
    // Person -> pii-db directly. Clear violation.
    let mut model = Model::default();
    model.add_element(Element::new("p", ElementKind::Person, "User"));
    let mut db = Element::new("db", ElementKind::Container, "DB");
    db.data_classes.push("pii".into());
    model.add_element(db);
    model.add_relationship(Relationship {
        frm: "p".into(),
        to: "db".into(),
        label: "uses".into(),
        technology: None,
        order: None,
    });
    let violations = check(&model, Severity::Warning);
    assert!(violations.iter().any(|v| v.rule == "data-class-boundary"));
}

#[test]
fn data_class_boundary_encryption_tag_also_protects() {
    let mut model = Model::default();
    model.add_element(Element::new("u", ElementKind::Person, "User"));
    let mut api = Element::new("api", ElementKind::Container, "API");
    api.tags.push("encryption".into());
    model.add_element(api);
    let mut db = Element::new("db", ElementKind::Container, "DB");
    db.data_classes.push("pii".into());
    model.add_element(db);
    model.add_relationship(Relationship {
        frm: "u".into(),
        to: "api".into(),
        label: "uses".into(),
        technology: None,
        order: None,
    });
    model.add_relationship(Relationship {
        frm: "api".into(),
        to: "db".into(),
        label: "reads".into(),
        technology: None,
        order: None,
    });
    let violations = check(&model, Severity::Warning);
    assert!(!violations.iter().any(|v| v.rule == "data-class-boundary"));
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
            order: None,
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
        animation: Animation::default(),
        composite: None,
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

//! Fixture-driven integration tests for the analyze pipeline.
//!
//! Each fixture is a tiny realistic project tree under
//! `forge/tests/fixtures/analyze/`. Tests run the full `analyze()` pipeline
//! against the fixture and assert on the resulting Model (container ids,
//! technology labels, route catalogs, inferred infra, cross-container
//! relationships).
//!
//! Set `UPDATE_EXPECT=1` and rerun tests if you later add `.forge` snapshot
//! comparisons; for now assertions live in code to avoid HashMap iteration
//! nondeterminism in the DSL emitter.

use std::path::PathBuf;

use super::{analyze, AnalyzeConfig};
use crate::model::ElementKind;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/analyze")
        .join(name)
}

fn run(name: &str) -> crate::model::Model {
    let cfg = AnalyzeConfig {
        paths: vec![fixture(name)],
        ..AnalyzeConfig::default()
    };
    analyze(&cfg)
}

#[test]
fn rust_axum_fixture() {
    let m = run("rust-axum");

    let c = m
        .elements
        .get("payments-api")
        .expect("payments-api container");
    assert_eq!(c.kind, ElementKind::Container);
    assert_eq!(c.technology.as_deref(), Some("Rust / Axum"));
    assert!(c.tags.iter().any(|t| t == "inferred"));
    assert!(c.tags.iter().any(|t| t == "inferred:code"));

    // Routes extracted from main.rs via the semantic scanner.
    let catalog = m
        .api_catalogs
        .iter()
        .find(|c| c.container == "payments-api")
        .expect("catalog for payments-api");
    let paths: Vec<&str> = catalog.endpoints.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.contains(&"/payments"));
    assert!(paths.contains(&"/health"));
}

#[test]
fn node_next_fixture() {
    let m = run("node-next");
    let c = m.elements.get("storefront").expect("storefront container");
    assert!(
        c.technology
            .as_deref()
            .map(|t| t.contains("Next.js"))
            .unwrap_or(false),
        "expected Next.js in {:?}",
        c.technology
    );
}

#[test]
fn go_gin_fixture() {
    let m = run("go-gin");
    let c = m.elements.get("orders").expect("orders container");
    assert_eq!(c.technology.as_deref(), Some("Go / Gin"));

    let catalog = m
        .api_catalogs
        .iter()
        .find(|c| c.container == "orders")
        .expect("catalog for orders");
    assert!(catalog.endpoints.iter().any(|e| e.path == "/orders"));
}

#[test]
fn python_fastapi_fixture() {
    let m = run("python-fastapi");
    let c = m.elements.get("catalog").expect("catalog container");
    assert_eq!(c.technology.as_deref(), Some("Python / FastAPI"));

    let catalog = m
        .api_catalogs
        .iter()
        .find(|c| c.container == "catalog")
        .expect("api catalog for catalog");
    assert!(catalog.endpoints.iter().any(|e| e.path == "/items"));

    // MongoDB URL in source → inferred infra container and relationship.
    assert!(m.elements.contains_key("_inferred_mongodb"));
    assert!(m
        .relationships
        .iter()
        .any(|r| r.frm == "catalog" && r.to == "_inferred_mongodb"));
}

#[test]
fn cargo_monorepo_fixture() {
    let m = run("cargo-monorepo");

    // Workspace members become their own containers; root does not.
    assert!(m.elements.contains_key("api"));
    assert!(m.elements.contains_key("worker"));
    assert!(!m.elements.contains_key("cargo-monorepo"));

    let api = m.elements.get("api").unwrap();
    assert_eq!(api.technology.as_deref(), Some("Rust / Axum"));

    let worker = m.elements.get("worker").unwrap();
    assert_eq!(worker.technology.as_deref(), Some("Rust"));

    // Path-prefix attribution: worker source with redis literal → inferred
    // redis container linked to worker, not api.
    assert!(m.elements.contains_key("_inferred_redis"));
    assert!(m
        .relationships
        .iter()
        .any(|r| r.frm == "worker" && r.to == "_inferred_redis"));
    assert!(!m
        .relationships
        .iter()
        .any(|r| r.frm == "api" && r.to == "_inferred_redis"));
}

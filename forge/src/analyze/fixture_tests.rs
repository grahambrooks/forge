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

use super::merge;
use super::{analyze, AnalyzeConfig};
use crate::model::ElementKind;
use crate::parser;

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

#[test]
fn codeowners_attributes_containers_to_teams() {
    let m = run("codeowners");

    // All three containers were discovered by the code scanner.
    assert!(m.elements.contains_key("payments"));
    assert!(m.elements.contains_key("notifications"));
    assert!(m.elements.contains_key("release-cli"));

    // Teams are populated with the correct container ids.
    let payments_team = m
        .teams
        .iter()
        .find(|t| t.name == "payments-team")
        .expect("payments-team");
    assert!(payments_team.owns.contains(&"payments".to_string()));

    let notif_team = m
        .teams
        .iter()
        .find(|t| t.name == "notifications-team")
        .expect("notifications-team");
    assert!(notif_team.owns.contains(&"notifications".to_string()));

    let release_eng = m
        .teams
        .iter()
        .find(|t| t.name == "release-eng")
        .expect("release-eng");
    assert!(release_eng.owns.contains(&"release-cli".to_string()));

    // `platform` is the default fallback; it should NOT also own a service
    // that a more specific rule claims, because last-matching-rule wins.
    let platform = m.teams.iter().find(|t| t.name == "platform");
    if let Some(p) = platform {
        assert!(!p.owns.contains(&"payments".to_string()));
        assert!(!p.owns.contains(&"notifications".to_string()));
        assert!(!p.owns.contains(&"release-cli".to_string()));
    }
}

#[test]
fn k8s_deployment_env_reaches_container_and_correlate() {
    let m = run("k8s-correlated");

    // The Rust service becomes a Container via the code scanner, and
    // k8s.rs attaches env_provides from the Deployment's env: block.
    let orders = m.elements.get("orders").expect("orders container");
    let provides = orders
        .properties
        .get("forge:env_provides")
        .expect("env_provides mirrored onto orders container");
    assert!(provides.contains("DATABASE_URL"));
    assert!(provides.contains("REDIS_URL"));

    // The DeploymentNode also records its own env_provides, for
    // deployment-view dashboards that want the runtime-scoped truth.
    let node = m
        .elements
        .get("k8s.prod.orders")
        .expect("orders deployment node");
    let node_provides = node
        .properties
        .get("forge:env_provides")
        .expect("deployment node env_provides");
    assert!(node_provides.contains("DATABASE_URL"));
    assert!(node_provides.contains("REDIS_URL"));

    // Correlate should NOT emit a self-edge (orders reads and provides the
    // same vars), and without a separate provider the orders container has
    // no `uses (...)` edge to anywhere.
    assert!(!m
        .relationships
        .iter()
        .any(|r| r.frm == "orders" && r.to == "orders"));
}

#[test]
fn env_correlation_links_reader_to_docker_provider() {
    let m = run("env-correlated");

    // The `ledger` container was discovered by the code scanner from
    // Cargo.toml; docker-compose enriched it with DATABASE_URL/REDIS_URL
    // from its `environment:` block. Meanwhile the same source file reads
    // both vars via std::env::var.
    //
    // The correlate pass should notice that `ledger` *provides* DATABASE_URL
    // to itself and should NOT emit a self-edge; but it should link `db` and
    // `cache` to `ledger`… wait: in this fixture `ledger` is the consumer.
    // The semantic scanner records ledger's reads; docker records db+cache
    // provides. Correlate emits ledger -> db and ledger -> cache.
    assert!(m.elements.contains_key("ledger"));
    assert!(m.elements.contains_key("db"));
    assert!(m.elements.contains_key("cache"));

    let has_ledger_db = m
        .relationships
        .iter()
        .any(|r| r.frm == "ledger" && r.to == "db" && r.label.contains("DATABASE_URL"));
    let has_ledger_cache = m
        .relationships
        .iter()
        .any(|r| r.frm == "ledger" && r.to == "cache" && r.label.contains("REDIS_URL"));
    assert!(has_ledger_db, "expected ledger -> db correlation");
    assert!(has_ledger_cache, "expected ledger -> cache correlation");

    // The concrete db edge should supersede any `_inferred_postgresql`
    // relationship the semantic scanner may have emitted from literal
    // strings. The fixture source has no postgres:// literal, but this
    // invariant still holds: the only edge from ledger to a database-tagged
    // container should be the correlated one.
    let db_edges: Vec<&str> = m
        .relationships
        .iter()
        .filter(|r| r.frm == "ledger" && r.to.starts_with("_inferred_"))
        .map(|r| r.to.as_str())
        .collect();
    assert!(
        db_edges.is_empty(),
        "expected no stale _inferred_* edges, got {:?}",
        db_edges
    );
}

#[test]
fn merge_preserves_user_content_and_refreshes_inferred() {
    // Start from a hand-authored .forge that mixes a user-owned system with
    // one stale inferred container (left over from a previous analyze run).
    // Running `analyze --merge` should preserve the user system plus its
    // relationship, drop the stale inferred container, and repopulate fresh
    // inferred entries from the cargo-monorepo fixture.
    let existing_src = r#"
forge "monorepo" {
  model {
    user = person "User"

    billing = system "Billing System" {
      description "Hand-authored upstream that analyze must never touch"
    }

    api = container "api" {
      description "User overrides for the api container"
      technology "Rust / Custom"
      tags "inferred" "inferred:code"
    }

    stale = container "Stale Service" {
      technology "PostgreSQL"
      tags "inferred" "inferred:code"
    }

    user -> billing "uses"
    billing -> stale "reads"
  }
}
"#;

    let mut existing = parser::parse(existing_src).expect("parse existing model");
    assert!(existing.elements.contains_key("stale"));

    let cfg = AnalyzeConfig {
        paths: vec![fixture("cargo-monorepo")],
        ..AnalyzeConfig::default()
    };
    let fresh = analyze(&cfg);
    merge::merge(&mut existing, fresh);

    // User-owned elements survive untouched.
    let billing = existing.elements.get("billing").expect("billing survives");
    assert_eq!(billing.name, "Billing System");
    assert_eq!(
        billing.description.as_deref(),
        Some("Hand-authored upstream that analyze must never touch")
    );
    assert!(existing.elements.contains_key("user"));

    // User relationship to a user element survives.
    assert!(existing
        .relationships
        .iter()
        .any(|r| r.frm == "user" && r.to == "billing"));

    // The stale inferred container is gone, along with the relationship
    // that pointed at it.
    assert!(!existing.elements.contains_key("stale"));
    assert!(!existing.relationships.iter().any(|r| r.to == "stale"));

    // Fresh inferred elements from the fixture are present.
    assert!(existing.elements.contains_key("worker"));
    assert!(existing.elements.contains_key("_inferred_redis"));

    // Id collision rule: the `api` container in the existing model was
    // tagged inferred, so the fresh analyzer's `api` replaces it. The fresh
    // entry has technology "Rust / Axum" from the fixture's Cargo.toml.
    let api = existing.elements.get("api").expect("api container");
    assert_eq!(api.technology.as_deref(), Some("Rust / Axum"));
}

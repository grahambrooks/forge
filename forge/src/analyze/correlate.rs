//! Cross-scanner correlation pass.
//!
//! Individual scanners produce narrow, scanner-local facts. This pass runs
//! after every scanner has written into the Model and upgrades those facts
//! into proper relationships where evidence from multiple scanners lines up.
//!
//! Correlations, in order:
//!
//! 1. **Exact env var match.** The semantic scanner records reads under
//!    `forge:env_reads`; docker/k8s record declarations under
//!    `forge:env_provides`. Shared variable names emit a concrete `uses`
//!    relationship from the reader to the provider. Database-tagged
//!    providers supersede any stale `_inferred_*` edges from the same
//!    reader.
//!
//! 2. **Connection-string fallback.** When a reader has an env var that
//!    *looks like* a data-store URL (DATABASE_URL, REDIS_URL, MONGO_URL, …)
//!    and no exact match fired, link the reader to the nearest existing
//!    container tagged `database` or `messaging` whose technology matches
//!    the expected kind. This handles the realistic case where a postgres
//!    service declares POSTGRES_PASSWORD rather than DATABASE_URL — the
//!    reader still knows what it's talking to.
//!
//! 3. **Pipeline stages → environments.** CI stages that declare
//!    `environment: prod` get linked to a synthetic `Environment` element
//!    (created on demand, one per unique name). When the environment name
//!    matches a k8s namespace that has DeploymentNodes, the environment
//!    is also linked to those deployments via a `hosts` relationship, so
//!    the generated pipeline view shows "this stage deploys to this env
//!    which runs these pods."

use std::collections::{HashMap, HashSet};

use crate::model::{Element, ElementKind, Model, Relationship};

use super::provenance::mark_inferred;
use super::slugify;

const ENV_READS_KEY: &str = "forge:env_reads";
const ENV_PROVIDES_KEY: &str = "forge:env_provides";

pub fn run(model: &mut Model) {
    correlate_env_vars(model);
    correlate_connection_strings(model);
    correlate_pipeline_environments(model);
}

fn correlate_env_vars(model: &mut Model) {
    // Snapshot reads and provides so we can mutate the model while iterating.
    let reads: HashMap<String, HashSet<String>> = model
        .elements
        .iter()
        .filter_map(|(id, el)| {
            el.properties
                .get(ENV_READS_KEY)
                .map(|s| (id.clone(), split_csv(s)))
        })
        .collect();

    let provides: HashMap<String, HashSet<String>> = model
        .elements
        .iter()
        .filter_map(|(id, el)| {
            el.properties
                .get(ENV_PROVIDES_KEY)
                .map(|s| (id.clone(), split_csv(s)))
        })
        .collect();

    if reads.is_empty() || provides.is_empty() {
        return;
    }

    // (reader_id, provider_id, shared var names)
    let mut links: Vec<(String, String, Vec<String>)> = Vec::new();
    for (reader, read_set) in &reads {
        for (provider, provide_set) in &provides {
            if reader == provider {
                continue;
            }
            let shared: Vec<String> = read_set.intersection(provide_set).cloned().collect();
            if !shared.is_empty() {
                links.push((reader.clone(), provider.clone(), shared));
            }
        }
    }

    for (reader, provider, shared) in links {
        let label = format!("uses ({})", shared.join(", "));
        // Coexist with other edges between the same pair (e.g. docker-compose
        // `depends_on` already emits a "depends on" edge). Only dedupe against
        // another correlation edge.
        let exists = model
            .relationships
            .iter()
            .any(|r| r.frm == reader && r.to == provider && r.label.starts_with("uses ("));
        if !exists {
            model.add_relationship(Relationship {
                frm: reader.clone(),
                to: provider.clone(),
                label,
                technology: None,
                order: None,
            });
        }

        // If the provider is a database, drop any stale _inferred_* edge from
        // the same reader pointing at a matching-kind inferred container.
        let provider_is_db = model
            .elements
            .get(&provider)
            .map(|el| el.tags.iter().any(|t| t == "database"))
            .unwrap_or(false);
        if provider_is_db {
            model
                .relationships
                .retain(|r| !(r.frm == reader && r.to.starts_with("_inferred_")));
        }
    }
}

fn split_csv(s: &str) -> HashSet<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

// ── Connection-string fallback ──────────────────────────────────────

/// Known `kind → env-var-names` hints. When a reader's env var names contain
/// any of the listed identifiers, we look for an existing container tagged
/// `database` or `messaging` whose technology indicates the same kind and
/// emit a `uses` relationship. First kind wins per variable name.
const CONNECTION_HINTS: &[(&str, &[&str])] = &[
    (
        "postgres",
        &[
            "DATABASE_URL",
            "DB_URL",
            "POSTGRES_URL",
            "POSTGRESQL_URL",
            "PG_URL",
            "PGHOST",
            "PGDATABASE",
        ],
    ),
    (
        "mysql",
        &["MYSQL_URL", "MYSQL_HOST", "MYSQL_DATABASE", "MARIADB_URL"],
    ),
    (
        "redis",
        &["REDIS_URL", "REDIS_HOST", "CACHE_URL", "CACHE_REDIS_URL"],
    ),
    (
        "mongo",
        &["MONGO_URL", "MONGODB_URI", "MONGO_URI", "MONGODB_URL"],
    ),
    (
        "elasticsearch",
        &["ELASTICSEARCH_URL", "ELASTIC_URL", "ES_URL"],
    ),
    ("kafka", &["KAFKA_BROKERS", "KAFKA_URL", "KAFKA_BOOTSTRAP"]),
    ("rabbitmq", &["AMQP_URL", "RABBITMQ_URL", "RABBITMQ_HOST"]),
];

fn correlate_connection_strings(model: &mut Model) {
    // Collect readers, their env_reads, and any env vars that are *already*
    // satisfied by an exact-match edge from the first pass (to avoid
    // duplicating work).
    let reads: Vec<(String, HashSet<String>)> = model
        .elements
        .iter()
        .filter_map(|(id, el)| {
            el.properties
                .get(ENV_READS_KEY)
                .map(|s| (id.clone(), split_csv(s)))
        })
        .collect();

    if reads.is_empty() {
        return;
    }

    // Build a kind → container_id index from existing database/messaging
    // elements. Technology string matching is substring-based and
    // case-insensitive so "PostgreSQL 16" and "Redis 7" both resolve.
    let provider_by_kind: HashMap<&'static str, Vec<String>> = {
        let mut m: HashMap<&'static str, Vec<String>> = HashMap::new();
        for (kind, _) in CONNECTION_HINTS {
            for (id, el) in &model.elements {
                if el.kind != ElementKind::Container {
                    continue;
                }
                let is_infra = el.tags.iter().any(|t| t == "database" || t == "messaging");
                if !is_infra {
                    continue;
                }
                let tech = el.technology.as_deref().unwrap_or("").to_ascii_lowercase();
                let name = el.name.to_ascii_lowercase();
                let id_low = id.to_ascii_lowercase();
                if tech.contains(kind) || name.contains(kind) || id_low.contains(kind) {
                    m.entry(kind).or_default().push(id.clone());
                }
            }
        }
        m
    };

    if provider_by_kind.is_empty() {
        return;
    }

    // For each reader, pick at most one provider per kind it mentions. Skip
    // kinds where an exact-match edge already exists (labelled `uses (VAR`)
    // so we don't double-count.
    let mut new_edges: Vec<(String, String, String)> = Vec::new();
    for (reader, read_set) in &reads {
        for (kind, hint_vars) in CONNECTION_HINTS {
            let matched_var = hint_vars.iter().find(|v| read_set.contains(**v));
            let matched_var = match matched_var {
                Some(v) => *v,
                None => continue,
            };
            // Respect any existing correlation edge for this kind: if the
            // reader already has a `uses (...)` edge to a provider with a
            // matching kind, skip.
            let already_linked = model.relationships.iter().any(|r| {
                r.frm == *reader
                    && r.label.starts_with("uses (")
                    && matches_kind(model, &r.to, kind)
            });
            if already_linked {
                continue;
            }
            if let Some(candidates) = provider_by_kind.get(kind) {
                // Deterministic pick: lexicographically smallest id so
                // multi-provider fixtures don't flake on HashMap ordering.
                let mut sorted = candidates.clone();
                sorted.sort();
                if let Some(target) = sorted.first() {
                    if target == reader {
                        continue;
                    }
                    new_edges.push((
                        reader.clone(),
                        target.clone(),
                        format!("uses ({matched_var})"),
                    ));
                }
            }
        }
    }

    for (frm, to, label) in new_edges {
        let exists = model
            .relationships
            .iter()
            .any(|r| r.frm == frm && r.to == to && r.label == label);
        if !exists {
            model.add_relationship(Relationship {
                frm: frm.clone(),
                to: to.clone(),
                label,
                technology: None,
                order: None,
            });
        }
        // Drop any stale _inferred_* edge from the reader now that a concrete
        // target exists — mirrors the exact-match pass.
        model
            .relationships
            .retain(|r| !(r.frm == frm && r.to.starts_with("_inferred_")));
    }
}

// ── Pipeline stage → environment ────────────────────────────────────

fn correlate_pipeline_environments(model: &mut Model) {
    // (stage_id, environment_name) for every Stage with an env declaration.
    let stage_envs: Vec<(String, String)> = model
        .elements
        .iter()
        .filter(|(_, el)| el.kind == ElementKind::Stage)
        .filter_map(|(id, el)| {
            el.properties
                .get("environment")
                .map(|e| (id.clone(), e.clone()))
        })
        .collect();

    if stage_envs.is_empty() {
        return;
    }

    // namespace → DeploymentNode ids, so an Environment named "prod" can be
    // linked to every k8s pod that runs in the `prod` namespace.
    let mut deployments_by_ns: HashMap<String, Vec<String>> = HashMap::new();
    for (id, el) in &model.elements {
        if el.kind != ElementKind::DeploymentNode {
            continue;
        }
        if let Some(ns) = el.properties.get("namespace") {
            deployments_by_ns
                .entry(ns.clone())
                .or_default()
                .push(id.clone());
        }
    }

    // First pass: create (or find) one Environment element per unique name.
    let mut env_id_by_name: HashMap<String, String> = HashMap::new();
    let unique_names: HashSet<String> = stage_envs.iter().map(|(_, e)| e.clone()).collect();

    for name in unique_names {
        let env_id = format!("env.{}", slugify(&name));
        env_id_by_name.insert(name.clone(), env_id.clone());

        if !model.elements.contains_key(&env_id) {
            let mut el = Element::new(&env_id, ElementKind::Environment, &name);
            el.description = Some("Deployment environment inferred from CI stages".to_string());
            mark_inferred(&mut el, "correlate", None);
            model.add_element(el);
        }

        // Link the environment to every DeploymentNode in a matching namespace.
        if let Some(deployments) = deployments_by_ns.get(&name) {
            for dep_id in deployments {
                let exists = model
                    .relationships
                    .iter()
                    .any(|r| r.frm == env_id && r.to == *dep_id && r.label == "hosts");
                if !exists {
                    model.add_relationship(Relationship {
                        frm: env_id.clone(),
                        to: dep_id.clone(),
                        label: "hosts".into(),
                        technology: None,
                        order: None,
                    });
                }
            }
        }
    }

    // Second pass: link each stage to its environment.
    for (stage_id, env_name) in stage_envs {
        let env_id = match env_id_by_name.get(&env_name) {
            Some(id) => id.clone(),
            None => continue,
        };
        let exists = model
            .relationships
            .iter()
            .any(|r| r.frm == stage_id && r.to == env_id && r.label == "deploys to");
        if !exists {
            model.add_relationship(Relationship {
                frm: stage_id,
                to: env_id,
                label: "deploys to".into(),
                technology: None,
                order: None,
            });
        }
    }
}

fn matches_kind(model: &Model, container_id: &str, kind: &str) -> bool {
    let el = match model.elements.get(container_id) {
        Some(e) => e,
        None => return false,
    };
    let tech = el.technology.as_deref().unwrap_or("").to_ascii_lowercase();
    tech.contains(kind)
        || el.name.to_ascii_lowercase().contains(kind)
        || container_id.to_ascii_lowercase().contains(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Element, ElementKind, Model};

    fn container(id: &str) -> Element {
        Element::new(id, ElementKind::Container, id)
    }

    #[test]
    fn links_reader_to_provider_by_shared_var() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "DATABASE_URL,OTHER".into());
        model.add_element(reader);

        let mut provider = container("postgres");
        provider.tags.push("database".into());
        provider.properties.insert(
            ENV_PROVIDES_KEY.into(),
            "DATABASE_URL,POSTGRES_PASSWORD".into(),
        );
        model.add_element(provider);

        run(&mut model);

        let rel = model
            .relationships
            .iter()
            .find(|r| r.frm == "api" && r.to == "postgres")
            .expect("correlation emitted");
        assert!(rel.label.contains("DATABASE_URL"));
    }

    #[test]
    fn db_provider_supersedes_inferred_edge() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "DATABASE_URL".into());
        model.add_element(reader);

        let mut inferred = container("_inferred_postgresql");
        inferred.tags.push("database".into());
        inferred.tags.push("inferred".into());
        model.add_element(inferred);

        // Pre-existing stale edge from the semantic scanner.
        model.add_relationship(Relationship {
            frm: "api".into(),
            to: "_inferred_postgresql".into(),
            label: "reads/writes".into(),
            technology: None,
            order: None,
        });

        let mut provider = container("postgres");
        provider.tags.push("database".into());
        provider
            .properties
            .insert(ENV_PROVIDES_KEY.into(), "DATABASE_URL".into());
        model.add_element(provider);

        run(&mut model);

        // Stale inferred edge is gone.
        assert!(!model
            .relationships
            .iter()
            .any(|r| r.frm == "api" && r.to == "_inferred_postgresql"));
        // Concrete edge replaces it.
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "api" && r.to == "postgres"));
    }

    #[test]
    fn no_links_without_shared_vars() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader.properties.insert(ENV_READS_KEY.into(), "FOO".into());
        model.add_element(reader);

        let mut provider = container("postgres");
        provider
            .properties
            .insert(ENV_PROVIDES_KEY.into(), "BAR".into());
        model.add_element(provider);

        run(&mut model);

        assert!(model.relationships.is_empty());
    }

    #[test]
    fn connection_string_fallback_picks_database_container() {
        let mut model = Model::default();

        // Reader wants DATABASE_URL but postgres only declares
        // POSTGRES_PASSWORD, which is the realistic shape.
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "DATABASE_URL".into());
        model.add_element(reader);

        let mut postgres = Element::new("pg", ElementKind::Container, "PostgreSQL");
        postgres.tags.push("database".into());
        postgres.technology = Some("PostgreSQL 16".into());
        postgres
            .properties
            .insert(ENV_PROVIDES_KEY.into(), "POSTGRES_PASSWORD".into());
        model.add_element(postgres);

        run(&mut model);

        let edge = model
            .relationships
            .iter()
            .find(|r| r.frm == "api" && r.to == "pg")
            .expect("fallback emitted edge");
        assert_eq!(edge.label, "uses (DATABASE_URL)");
    }

    #[test]
    fn fallback_resolves_redis_by_technology_substring() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "REDIS_URL".into());
        model.add_element(reader);

        let mut redis = Element::new("cache", ElementKind::Container, "Cache");
        redis.tags.push("database".into());
        redis.technology = Some("Redis".into());
        model.add_element(redis);

        run(&mut model);

        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "api" && r.to == "cache" && r.label == "uses (REDIS_URL)"));
    }

    #[test]
    fn fallback_skipped_when_exact_match_already_fired() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "DATABASE_URL".into());
        model.add_element(reader);

        // A compose-style provider that explicitly declares DATABASE_URL.
        let mut provider = Element::new("db", ElementKind::Container, "db");
        provider.tags.push("database".into());
        provider.technology = Some("Postgres".into());
        provider
            .properties
            .insert(ENV_PROVIDES_KEY.into(), "DATABASE_URL".into());
        model.add_element(provider);

        // A second postgres container that would qualify for the fallback
        // but shouldn't get a duplicate edge from the same reader.
        let mut shadow = Element::new("shadow-pg", ElementKind::Container, "Shadow");
        shadow.tags.push("database".into());
        shadow.technology = Some("PostgreSQL".into());
        model.add_element(shadow);

        run(&mut model);

        let db_edges: Vec<&str> = model
            .relationships
            .iter()
            .filter(|r| r.frm == "api" && r.label.starts_with("uses ("))
            .map(|r| r.to.as_str())
            .collect();
        assert_eq!(db_edges, vec!["db"]);
    }

    #[test]
    fn pipeline_stage_linked_to_environment_and_k8s_namespace() {
        let mut model = Model::default();

        // A Stage declaring `environment: prod`.
        let mut stage = Element::new("deploy.prod-job", ElementKind::Stage, "Deploy prod");
        stage.properties.insert("environment".into(), "prod".into());
        model.add_element(stage);

        // A DeploymentNode running in the `prod` namespace.
        let mut dep = Element::new("k8s.prod.api", ElementKind::DeploymentNode, "api");
        dep.properties.insert("namespace".into(), "prod".into());
        model.add_element(dep);

        run(&mut model);

        // Environment element was synthesised.
        let env = model.elements.get("env.prod").expect("env.prod created");
        assert_eq!(env.kind, ElementKind::Environment);
        assert!(env.tags.iter().any(|t| t == "inferred"));

        // Stage → env edge
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "deploy.prod-job" && r.to == "env.prod" && r.label == "deploys to"));

        // Env → deployment edge
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "env.prod" && r.to == "k8s.prod.api" && r.label == "hosts"));
    }

    #[test]
    fn pipeline_env_synth_even_without_k8s() {
        let mut model = Model::default();
        let mut stage = Element::new("deploy.staging-job", ElementKind::Stage, "Deploy staging");
        stage
            .properties
            .insert("environment".into(), "staging".into());
        model.add_element(stage);

        run(&mut model);

        assert!(model.elements.contains_key("env.staging"));
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "deploy.staging-job" && r.to == "env.staging"));
    }

    #[test]
    fn fallback_does_nothing_when_no_database_containers_exist() {
        let mut model = Model::default();
        let mut reader = container("api");
        reader
            .properties
            .insert(ENV_READS_KEY.into(), "DATABASE_URL".into());
        model.add_element(reader);

        run(&mut model);
        assert!(model.relationships.is_empty());
    }
}

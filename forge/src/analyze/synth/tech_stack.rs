//! Aggregates container technology labels into a `tech_stack` view organised
//! by architectural layer: App → Service → Persistence → Infrastructure.

use std::collections::BTreeMap;

use crate::model::*;

// ── Layer name constants ─────────────────────────────────────────
pub(super) const LAYER_APP: &str = "App";
pub(super) const LAYER_SERVICE: &str = "Service";
pub(super) const LAYER_PERSISTENCE: &str = "Persistence";
pub(super) const LAYER_INFRASTRUCTURE: &str = "Infrastructure";

/// Aggregate container technology labels into tech_stack categories organised by
/// architectural layer: App → Service → Persistence → Infrastructure.
pub(super) fn synthesize_tech_stack(model: &mut Model) {
    if !model.tech_stack.is_empty() {
        return;
    }

    // layer → deduplicated list of (tech_name, purpose_label)
    let mut by_layer: BTreeMap<&'static str, Vec<(String, String)>> = BTreeMap::new();

    for el in model.elements.values() {
        if el.kind != ElementKind::Container {
            continue;
        }
        let tech = match el.technology.as_deref() {
            Some(t) if !t.is_empty() => t,
            _ => continue,
        };

        // Technology labels produced by the code scanner use "Language / Framework".
        // Docker/infra scanners emit bare names like "PostgreSQL" or "Redis".
        match tech.split_once(" / ") {
            Some((language, framework)) => {
                let language = language.trim();
                let framework = framework.trim();
                let layer = classify_tech_layer(framework);
                let entries = by_layer.entry(layer).or_default();
                if !entries.iter().any(|(n, _)| n == framework) {
                    entries.push((framework.to_string(), format!("{language} framework")));
                }
            }
            None => {
                // Bare technology name (database image, platform tool, language only)
                let name = tech.trim();
                let layer = classify_tech_layer(name);
                let entries = by_layer.entry(layer).or_default();
                if !entries.iter().any(|(n, _)| n == name) {
                    entries.push((name.to_string(), layer_purpose_label(layer)));
                }
            }
        }
    }

    // Emit categories in a stable, meaningful order.
    for layer in [
        LAYER_APP,
        LAYER_SERVICE,
        LAYER_PERSISTENCE,
        LAYER_INFRASTRUCTURE,
    ] {
        if let Some(entries) = by_layer.get(layer) {
            model.tech_stack.push(TechCategory {
                name: layer.to_string(),
                entries: entries
                    .iter()
                    .map(|(name, purpose)| TechEntry {
                        name: name.clone(),
                        version: None,
                        purpose: Some(purpose.clone()),
                    })
                    .collect(),
            });
        }
    }
}

/// Classify a technology name into one of the four architectural layers.
pub(super) fn classify_tech_layer(tech: &str) -> &'static str {
    // Frontend / UI frameworks → App
    const APP_TECHS: &[&str] = &[
        "React", "Angular", "Vue", "Svelte", "Next.js", "Nuxt", "Remix", "Gatsby", "Astro",
        "Ember", "Solid", "Lit", "Preact", "Flutter", "Blazor",
    ];
    // Databases, caches, message queues → Persistence
    const PERSISTENCE_TECHS: &[&str] = &[
        "PostgreSQL",
        "MySQL",
        "MariaDB",
        "SQLite",
        "Oracle",
        "MSSQL",
        "SQL Server",
        "MongoDB",
        "Cassandra",
        "CouchDB",
        "DynamoDB",
        "Firestore",
        "Cosmos DB",
        "Redis",
        "Memcached",
        "Apache Kafka",
        "Kafka",
        "RabbitMQ",
        "NATS",
        "ActiveMQ",
        "SQS",
        "Elasticsearch",
        "OpenSearch",
        "InfluxDB",
        "TimescaleDB",
        "Neo4j",
    ];
    // Platform, cloud, container runtime, IaC, reverse proxies → Infrastructure
    const INFRA_TECHS: &[&str] = &[
        "Docker",
        "Kubernetes",
        "K8s",
        "Helm",
        "AWS",
        "GCP",
        "Azure",
        "Terraform",
        "Pulumi",
        "Ansible",
        "Chef",
        "Puppet",
        "Nginx",
        "Traefik",
        "HAProxy",
        "Envoy",
        "Linkerd",
        "Istio",
        "Prometheus",
        "Grafana",
        "Datadog",
        "CloudWatch",
        "Jaeger",
        "Zipkin",
        "OpenTelemetry",
    ];

    if APP_TECHS.iter().any(|a| tech_name_matches(tech, a)) {
        return LAYER_APP;
    }
    if PERSISTENCE_TECHS.iter().any(|p| tech_name_matches(tech, p)) {
        return LAYER_PERSISTENCE;
    }
    if INFRA_TECHS.iter().any(|i| tech_name_matches(tech, i)) {
        return LAYER_INFRASTRUCTURE;
    }
    // Backend service frameworks and plain languages default to Service.
    LAYER_SERVICE
}

/// Match a technology label against a known pattern using whole-word semantics.
///
/// Exact equality is checked first. Versioned labels like "PostgreSQL 16" or
/// "Redis 7" are matched by checking that `tech` starts with `pattern` and the
/// next character (if any) is a space — this avoids false-positive substring
/// matches such as "MyReactiveService" matching "React".
fn tech_name_matches(tech: &str, pattern: &str) -> bool {
    if tech.eq_ignore_ascii_case(pattern) {
        return true;
    }
    // Allow "Pattern <version-or-extra>" (e.g. "PostgreSQL 16" matches "PostgreSQL")
    if tech.len() > pattern.len() {
        let (prefix, rest) = tech.split_at(pattern.len());
        if prefix.eq_ignore_ascii_case(pattern) && rest.starts_with(' ') {
            return true;
        }
    }
    false
}

/// A short human-readable purpose label for bare technology names placed
/// in a layer by `classify_tech_layer`.
fn layer_purpose_label(layer: &str) -> String {
    match layer {
        LAYER_APP => "frontend".into(),
        LAYER_SERVICE => "service".into(),
        LAYER_PERSISTENCE => "data store".into(),
        LAYER_INFRASTRUCTURE => "infrastructure".into(),
        _ => "technology".into(),
    }
}

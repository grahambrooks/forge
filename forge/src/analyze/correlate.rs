//! Cross-scanner correlation pass.
//!
//! Individual scanners produce narrow, scanner-local facts. This pass runs
//! after every scanner has written into the Model and upgrades those facts
//! into proper relationships where evidence from multiple scanners lines up.
//!
//! Currently it handles one correlation:
//!
//! **Env var consumers ↔ providers.** The semantic scanner records which
//! environment variables each container *reads* (`forge:env_reads`). The
//! docker scanner records which variables each service *provides*
//! (`forge:env_provides`). When a read and a provide agree on the same
//! variable name, we emit a concrete `uses` relationship from the reader to
//! the provider. If the provider is tagged `database`, any previously
//! inferred `_inferred_postgresql` (etc.) edge from the reader is dropped in
//! favour of the concrete edge.

use std::collections::{HashMap, HashSet};

use crate::model::{Model, Relationship};

const ENV_READS_KEY: &str = "forge:env_reads";
const ENV_PROVIDES_KEY: &str = "forge:env_provides";

pub fn run(model: &mut Model) {
    correlate_env_vars(model);
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
}

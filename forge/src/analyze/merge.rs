//! Merge a fresh analyze() result into an existing hand-authored Model.
//!
//! Rule: anything tagged `inferred` in the existing model is the analyzer's
//! territory and gets refreshed. Everything else is user content and is left
//! alone. This is what makes `forge analyze --merge` safe to re-run in CI —
//! regenerated containers never trample descriptions or relationships that
//! a human added.
//!
//! Scope:
//!   - Elements: inferred entries replaced by fresh ones; user entries kept.
//!     Fresh elements whose id collides with an existing user entry are
//!     dropped (user wins).
//!   - Relationships: any relationship referencing an inferred element in
//!     the existing model is implicitly removed (its endpoint is gone).
//!     Fresh relationships are added if both endpoints resolve in the merged
//!     model and no equivalent relationship already exists.
//!   - API catalogs: for every container the fresh scan populated, replace
//!     the existing catalog entry wholesale. Other catalogs are preserved.
//!
//! Relationships aren't tagged today — the "endpoint-is-inferred" heuristic
//! is enough because inferred edges always have at least one inferred side.

use std::collections::HashSet;

use crate::model::{Element, Model};

use super::provenance::INFERRED_TAG;

pub fn merge(existing: &mut Model, fresh: Model) {
    // ── 1. Drop inferred elements from the existing model ──
    let removed: HashSet<String> = existing
        .elements
        .iter()
        .filter(|(_, el)| is_inferred(el))
        .map(|(id, _)| id.clone())
        .collect();
    existing.elements.retain(|id, _| !removed.contains(id));

    // Clean up dangling child references on any surviving parents.
    for el in existing.elements.values_mut() {
        el.children.retain(|c| !removed.contains(c));
    }

    // ── 2. Drop relationships whose endpoints no longer exist ──
    existing.relationships.retain(|r| {
        existing.elements.contains_key(&r.frm) && existing.elements.contains_key(&r.to)
    });

    // ── 3. Add fresh inferred elements, letting user ids win on collision ──
    for (id, el) in fresh.elements {
        if !is_inferred(&el) {
            // Analyzer only produces inferred elements today. Belt and braces:
            // skip anything that somehow isn't tagged so we never inject
            // un-owned entries into a merged model.
            continue;
        }
        existing.elements.entry(id).or_insert(el);
    }

    // ── 4. Add fresh relationships (dedup, resolve, skip unknowns) ──
    for rel in fresh.relationships {
        let both_known =
            existing.elements.contains_key(&rel.frm) && existing.elements.contains_key(&rel.to);
        if !both_known {
            continue;
        }
        let exists = existing
            .relationships
            .iter()
            .any(|r| r.frm == rel.frm && r.to == rel.to && r.label == rel.label);
        if !exists {
            existing.relationships.push(rel);
        }
    }

    // ── 5. Replace API catalogs for containers the fresh scan touched ──
    let fresh_containers: HashSet<String> = fresh
        .api_catalogs
        .iter()
        .map(|c| c.container.clone())
        .collect();
    existing
        .api_catalogs
        .retain(|c| !fresh_containers.contains(&c.container));
    existing.api_catalogs.extend(fresh.api_catalogs);
}

fn is_inferred(el: &Element) -> bool {
    el.tags
        .iter()
        .any(|t| t == INFERRED_TAG || t.starts_with(&format!("{INFERRED_TAG}:")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ApiCatalog, ApiEndpoint, Element, ElementKind, Model, Relationship};

    fn inferred(id: &str, name: &str) -> Element {
        let mut el = Element::new(id, ElementKind::Container, name);
        el.tags.push(INFERRED_TAG.into());
        el.tags.push("inferred:code".into());
        el
    }

    fn user(id: &str, name: &str) -> Element {
        Element::new(id, ElementKind::Container, name)
    }

    #[test]
    fn user_elements_survive_merge() {
        let mut existing = Model::default();
        existing.add_element(user("payments", "Payments"));
        existing.add_element(inferred("stale-service", "Stale"));
        existing.add_relationship(Relationship {
            frm: "payments".into(),
            to: "stale-service".into(),
            label: "calls".into(),
            technology: None,
        });

        let mut fresh = Model::default();
        fresh.add_element(inferred("new-service", "New"));
        fresh.add_relationship(Relationship {
            frm: "payments".into(),
            to: "new-service".into(),
            label: "calls".into(),
            technology: None,
        });

        merge(&mut existing, fresh);

        assert!(existing.elements.contains_key("payments"));
        assert!(!existing.elements.contains_key("stale-service"));
        assert!(existing.elements.contains_key("new-service"));
        // The stale relationship was dropped because its target vanished.
        assert!(existing.relationships.iter().any(|r| r.to == "new-service"));
        assert!(!existing
            .relationships
            .iter()
            .any(|r| r.to == "stale-service"));
    }

    #[test]
    fn user_id_wins_on_collision() {
        let mut existing = Model::default();
        let mut el = user("api", "Hand-Authored API");
        el.description = Some("user wrote this".into());
        existing.add_element(el);

        let mut fresh = Model::default();
        let mut clash = inferred("api", "Inferred API");
        clash.description = Some("analyzer wrote this".into());
        fresh.add_element(clash);

        merge(&mut existing, fresh);

        let kept = existing.elements.get("api").unwrap();
        assert_eq!(kept.name, "Hand-Authored API");
        assert_eq!(kept.description.as_deref(), Some("user wrote this"));
        // User element was NOT tagged inferred; ensure we didn't leak the tag.
        assert!(!kept.tags.iter().any(|t| t == INFERRED_TAG));
    }

    #[test]
    fn api_catalogs_refreshed_for_touched_containers() {
        let mut existing = Model::default();
        existing.add_element(user("svc", "Service"));
        existing.api_catalogs.push(ApiCatalog {
            container: "svc".into(),
            endpoints: vec![ApiEndpoint {
                method: "GET".into(),
                path: "/stale".into(),
                description: None,
                request_body: None,
                response: None,
            }],
        });
        // Catalog on an untouched container should survive.
        existing.api_catalogs.push(ApiCatalog {
            container: "other".into(),
            endpoints: vec![ApiEndpoint {
                method: "GET".into(),
                path: "/keep".into(),
                description: None,
                request_body: None,
                response: None,
            }],
        });

        let mut fresh = Model::default();
        fresh.api_catalogs.push(ApiCatalog {
            container: "svc".into(),
            endpoints: vec![ApiEndpoint {
                method: "POST".into(),
                path: "/fresh".into(),
                description: None,
                request_body: None,
                response: None,
            }],
        });

        merge(&mut existing, fresh);

        let svc_cat = existing
            .api_catalogs
            .iter()
            .find(|c| c.container == "svc")
            .unwrap();
        assert_eq!(svc_cat.endpoints.len(), 1);
        assert_eq!(svc_cat.endpoints[0].path, "/fresh");

        let other_cat = existing
            .api_catalogs
            .iter()
            .find(|c| c.container == "other")
            .unwrap();
        assert_eq!(other_cat.endpoints[0].path, "/keep");
    }
}

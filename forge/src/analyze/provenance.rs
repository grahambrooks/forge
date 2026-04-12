//! Provenance helpers for inferred model elements.
//!
//! Every element produced by a scanner carries two conventions:
//! - a tag `inferred` so consumers can distinguish it from hand-authored elements
//! - a tag `inferred:<scanner>` identifying which scanner produced it
//! - an optional `forge:source` property recording the originating file path
//!
//! These live on the existing `Element.tags` / `Element.properties` so the DSL
//! emitter and downstream tooling require no changes.

use std::path::Path;

use crate::model::Element;

pub const INFERRED_TAG: &str = "inferred";

pub fn mark_inferred(el: &mut Element, scanner: &str, source: Option<&Path>) {
    push_unique(&mut el.tags, INFERRED_TAG.to_string());
    push_unique(&mut el.tags, format!("{INFERRED_TAG}:{scanner}"));
    if let Some(p) = source {
        el.properties
            .insert("forge:source".into(), p.display().to_string());
    }
}

fn push_unique(tags: &mut Vec<String>, tag: String) {
    if !tags.iter().any(|t| t == &tag) {
        tags.push(tag);
    }
}

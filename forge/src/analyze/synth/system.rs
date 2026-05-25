//! Wraps orphan top-level containers in a synthesized System element so that
//! `system-context` and `container` views have a meaningful scope.

use crate::model::*;

use super::SCANNER;
use crate::analyze::provenance::mark_inferred;
use crate::analyze::slugify;

/// Wrap top-level containers in a synthesized System when no System exists.
/// Hand-authored models with a System are left alone.
pub(super) fn ensure_system(model: &mut Model) {
    if model
        .elements
        .values()
        .any(|e| e.kind == ElementKind::System)
    {
        return;
    }

    let orphans: Vec<String> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Container && e.parent.is_none())
        .map(|e| e.id.clone())
        .collect();
    if orphans.is_empty() {
        return;
    }

    let sys_name = if model.name.is_empty() {
        "System".to_string()
    } else {
        model.name.clone()
    };
    let sys_id = slugify(&format!("{}-system", sys_name));
    // Don't clobber an existing id; bail if something already owns the slug.
    if model.elements.contains_key(&sys_id) {
        return;
    }

    let mut sys = Element::new(&sys_id, ElementKind::System, &sys_name);
    sys.description = Some(format!("Inferred system wrapper for {}", sys_name));
    mark_inferred(&mut sys, SCANNER, None);
    sys.children = orphans.clone();
    model.elements.insert(sys_id.clone(), sys);

    for cid in &orphans {
        if let Some(child) = model.elements.get_mut(cid) {
            child.parent = Some(sys_id.clone());
        }
    }
}

//! Synthesises an Animation for a dynamic view that has no explicit
//! `animation { frames }` block, drawing one frame per unique step number
//! from the ordered relationships in scope.

use crate::model::*;

/// For a dynamic view with no explicit animation, synthesize one frame per
/// unique step number drawn from the ordered relationships in the view's
/// scope. Each frame is cumulative: frame N includes every element and
/// relationship with `order <= N`, so a viewer stepping through the view
/// watches the flow progress from step 1 to step N.
///
/// Returns `None` when the view isn't dynamic, already carries an explicit
/// animation, or has no ordered relationships.
pub(super) fn derive_dynamic_animation(view: &View, model: &Model) -> Option<Animation> {
    if view.kind != ViewKind::Dynamic || !view.animation.is_empty() {
        return None;
    }

    // Collect ordered relationships whose endpoints are in scope. For a
    // dynamic view, scope is the `system` id; include everything whose
    // parent (recursively) rolls up to that system.
    let scope_id = view.scope.as_deref().unwrap_or("");
    let in_scope = |id: &str| -> bool {
        if id == scope_id {
            return true;
        }
        let mut cur = id;
        while let Some(el) = model.elements.get(cur) {
            if el.id == scope_id {
                return true;
            }
            match el.parent.as_deref() {
                Some(p) => cur = p,
                None => return false,
            }
        }
        false
    };

    let mut ordered: Vec<(&Relationship, u32)> = model
        .relationships
        .iter()
        .filter_map(|r| r.order.map(|o| (r, o)))
        .filter(|(r, _)| in_scope(&r.frm) && in_scope(&r.to))
        .collect();
    if ordered.is_empty() {
        return None;
    }
    ordered.sort_by_key(|(_, o)| *o);

    // Unique, sorted step numbers. Gaps are allowed; we emit one frame
    // per distinct value in ascending order.
    let mut seen_steps = Vec::new();
    for (_, o) in &ordered {
        if !seen_steps.contains(o) {
            seen_steps.push(*o);
        }
    }

    let mut frames = Vec::with_capacity(seen_steps.len());
    for &step in &seen_steps {
        let mut includes: Vec<String> = Vec::new();
        for (rel, o) in &ordered {
            if *o > step {
                break;
            }
            // Always include both endpoints so nodes appear at the same
            // time as the arrow that uses them.
            if !includes.contains(&rel.frm) {
                includes.push(rel.frm.clone());
            }
            if !includes.contains(&rel.to) {
                includes.push(rel.to.clone());
            }
            // Include the relationship itself as "frm -> to" so the edge
            // is revealed alongside its endpoints.
            let edge_key = format!("{} -> {}", rel.frm, rel.to);
            if !includes.contains(&edge_key) {
                includes.push(edge_key);
            }
        }
        frames.push(AnimationFrame {
            label: format!("Step {step}"),
            includes,
            include_all: false,
            highlights: Vec::new(),
            states: Vec::new(),
            notes: None,
        });
    }

    Some(Animation { frames })
}

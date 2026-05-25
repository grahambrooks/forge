//! Synthesises a default `github-flow` branching strategy with a `main` trunk
//! when `git.rs` did not find any branches in the scanned repository.

use crate::model::*;

use super::SCANNER;
use crate::analyze::provenance::mark_inferred;

/// Every analysed project should carry at least a trunk branch and a strategy
/// so the generated `.forge` renders a branching-view. When `git.rs` already
/// found branches (any real repo) we leave them alone; otherwise we fall back
/// to github-flow with a `main` trunk.
pub(super) fn synthesize_branching_strategy(model: &mut Model) {
    let has_branch = model
        .elements
        .values()
        .any(|e| e.kind == ElementKind::Branch);
    if has_branch {
        return;
    }

    let strategy_id = "github-flow";
    let trunk_id = format!("{strategy_id}.trunk");
    if model.elements.contains_key(&trunk_id) {
        return;
    }

    let mut trunk = Element::new(&trunk_id, ElementKind::Branch, "main");
    trunk.parent = Some(strategy_id.into());
    trunk
        .properties
        .insert("strategy".into(), strategy_id.into());
    trunk.tags.push("trunk".into());
    mark_inferred(&mut trunk, SCANNER, None);
    model.add_element(trunk);
}

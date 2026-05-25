//! Synthesis post-pass — fills gaps in inferred models so they render useful
//! views out of the box.
//!
//! Individual scanners detect facts narrowly: `code` emits containers,
//! `ci` emits pipelines, `docker` emits docker-derived containers. None of
//! them produces a System element, a Person actor, a tech-stack aggregate,
//! or Views — so a freshly analyzed `.forge` has plenty of raw material but
//! nothing to draw.
//!
//! This pass runs after `correlate::run()` (so every cross-scanner fact is
//! already merged) and fills those gaps:
//!
//! 1. Wraps orphan top-level containers in a synthesized System so the
//!    `system-context` and `container` views have a meaningful scope.
//! 2. Synthesizes a default User actor when any web-framework container is
//!    present and a Developer actor when any Pipeline is present, so the
//!    context view shows a plausible system boundary.
//! 3. Aggregates distinct `<language> / <framework>` technology labels
//!    from containers into `tech_stack` categories so `tech-stack-view`
//!    renders.
//! 4. Emits a default set of views keyed off element kinds actually
//!    present in the model — context, containers, components, pipeline,
//!    deployment, branching, tech-stack, teams, trust boundaries, data
//!    model. Hand-authored `views {}` blocks are never overwritten.

use crate::model::*;

mod branching;
mod persons;
mod system;
mod tech_stack;
mod views;

#[cfg(test)]
mod tests;

const SCANNER: &str = "synth";

pub fn run(model: &mut Model) {
    system::ensure_system(model);
    persons::synthesize_persons(model);
    branching::synthesize_branching_strategy(model);
    tech_stack::synthesize_tech_stack(model);
    views::synthesize_views(model);
}

fn unique_id(model: &Model, base: &str) -> String {
    if !model.elements.contains_key(base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !model.elements.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

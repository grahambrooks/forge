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

use super::provenance::mark_inferred;
use super::slugify;

const SCANNER: &str = "synth";

pub fn run(model: &mut Model) {
    ensure_system(model);
    synthesize_persons(model);
    synthesize_tech_stack(model);
    synthesize_views(model);
}

/// Wrap top-level containers in a synthesized System when no System exists.
/// Hand-authored models with a System are left alone.
fn ensure_system(model: &mut Model) {
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

/// Add a User and/or Developer actor when the model has evidence for them.
fn synthesize_persons(model: &mut Model) {
    if model
        .elements
        .values()
        .any(|e| e.kind == ElementKind::Person)
    {
        return;
    }

    let web_target: Option<String> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Container && is_web_tech(e.technology.as_deref()))
        .map(|e| e.id.clone())
        .next();

    let pipeline_id: Option<String> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Pipeline)
        .map(|e| e.id.clone())
        .next();

    if let Some(target) = web_target.clone() {
        let user_id = unique_id(model, "user");
        let mut user = Element::new(&user_id, ElementKind::Person, "User");
        user.description = Some("End user inferred from a web-facing container".to_string());
        mark_inferred(&mut user, SCANNER, None);
        model.elements.insert(user_id.clone(), user);
        model.add_relationship(Relationship {
            frm: user_id,
            to: target,
            label: "uses".into(),
            technology: None,
            order: None,
        });
    }

    if let Some(pid) = pipeline_id {
        let dev_id = unique_id(model, "developer");
        let mut dev = Element::new(&dev_id, ElementKind::Person, "Developer");
        dev.description = Some("Engineer inferred from CI pipeline presence".to_string());
        mark_inferred(&mut dev, SCANNER, None);
        model.elements.insert(dev_id.clone(), dev);
        model.add_relationship(Relationship {
            frm: dev_id,
            to: pid,
            label: "triggers".into(),
            technology: None,
            order: None,
        });
    }
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

/// Keyword match against common web-framework technology labels produced by
/// the code scanner's dependency inference (e.g. "Rust / Axum").
fn is_web_tech(tech: Option<&str>) -> bool {
    const KEYWORDS: &[&str] = &[
        "Axum",
        "Actix",
        "Rocket",
        "Warp",
        "Hyper",
        "Flask",
        "FastAPI",
        "Django",
        "Starlette",
        "Tornado",
        "Express",
        "Next.js",
        "Nest",
        "Fastify",
        "Koa",
        "Hapi",
        "Remix",
        "Nuxt",
        "Gin",
        "Echo",
        "Fiber",
        "Chi",
        "Beego",
        "Spring Boot",
        "Spring MVC",
        "Micronaut",
        "Quarkus",
        "Play",
        "Laravel",
        "Symfony",
        "Slim",
        "Rails",
        "Sinatra",
        "Hanami",
        "Phoenix",
        "ASP.NET",
    ];
    match tech {
        Some(s) => KEYWORDS.iter().any(|k| s.contains(k)),
        None => false,
    }
}

/// Aggregate container technology labels into tech_stack categories.
fn synthesize_tech_stack(model: &mut Model) {
    if !model.tech_stack.is_empty() {
        return;
    }

    use std::collections::BTreeMap;
    let mut by_language: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for el in model.elements.values() {
        if el.kind != ElementKind::Container {
            continue;
        }
        let tech = match el.technology.as_deref() {
            Some(t) if !t.is_empty() => t,
            _ => continue,
        };
        let (language, framework) = match tech.split_once(" / ") {
            Some((l, f)) => (l.trim().to_string(), Some(f.trim().to_string())),
            None => (tech.trim().to_string(), None),
        };
        let entry = by_language.entry(language).or_default();
        if let Some(f) = framework {
            if !entry.iter().any(|e| e == &f) {
                entry.push(f);
            }
        }
    }

    for (language, frameworks) in by_language {
        let mut entries: Vec<TechEntry> = Vec::new();
        entries.push(TechEntry {
            name: language.clone(),
            version: None,
            purpose: Some("language".into()),
        });
        for f in frameworks {
            entries.push(TechEntry {
                name: f,
                version: None,
                purpose: Some("framework".into()),
            });
        }
        model.tech_stack.push(TechCategory {
            name: language,
            entries,
        });
    }
}

/// Emit default views keyed off the element kinds actually present in the
/// model. If the model already declares views we leave them alone.
fn synthesize_views(model: &mut Model) {
    if !model.views.is_empty() {
        return;
    }

    let system_id = first_id(model, ElementKind::System);
    let container_id = first_id(model, ElementKind::Container);
    let component_id = first_id(model, ElementKind::Component);
    let pipeline_id = first_id(model, ElementKind::Pipeline);
    let deployment_id = first_id(model, ElementKind::DeploymentNode);
    let branch_id = first_id(model, ElementKind::Branch);

    let mut views: Vec<View> = Vec::new();
    let context_scope = system_id.clone().or_else(|| container_id.clone());

    if let Some(scope) = context_scope.clone() {
        views.push(make_view(
            ViewKind::SystemContext,
            "SystemContext",
            Some(scope),
            "System Context",
            AutoLayout::LeftRight,
        ));
    }
    if let Some(scope) = system_id.clone().or_else(|| container_id.clone()) {
        views.push(make_view(
            ViewKind::Container,
            "Containers",
            Some(scope),
            "Containers",
            AutoLayout::TopBottom,
        ));
    }
    if let (Some(_), Some(c)) = (component_id.as_ref(), container_id.as_ref()) {
        views.push(make_view(
            ViewKind::Component,
            "Components",
            Some(c.clone()),
            "Components",
            AutoLayout::TopBottom,
        ));
    }
    if let Some(p) = pipeline_id {
        views.push(make_view(
            ViewKind::PipelineView,
            "Pipeline",
            Some(p),
            "CI/CD Pipeline",
            AutoLayout::LeftRight,
        ));
    }
    if let Some(d) = deployment_id {
        views.push(make_view(
            ViewKind::Deployment,
            "Deployment",
            Some(d),
            "Deployment",
            AutoLayout::TopBottom,
        ));
    }
    if let Some(b) = branch_id {
        views.push(make_view(
            ViewKind::Branching,
            "Branching",
            Some(b),
            "Branching Strategy",
            AutoLayout::TopBottom,
        ));
    }
    if !model.tech_stack.is_empty() {
        views.push(make_view(
            ViewKind::TechStack,
            "TechStack",
            None,
            "Technology Stack",
            AutoLayout::TopBottom,
        ));
    }
    if !model.teams.is_empty() {
        views.push(make_view(
            ViewKind::TeamMap,
            "Teams",
            None,
            "Team Ownership",
            AutoLayout::TopBottom,
        ));
    }
    if !model.trust_boundaries.is_empty() {
        views.push(make_view(
            ViewKind::TrustBoundaryView,
            "TrustBoundaries",
            None,
            "Trust Boundaries",
            AutoLayout::TopBottom,
        ));
    }
    if !model.data_entities.is_empty() {
        views.push(make_view(
            ViewKind::DataModel,
            "DataModel",
            None,
            "Data Model",
            AutoLayout::TopBottom,
        ));
    }
    if !model.api_catalogs.is_empty() {
        views.push(make_view(
            ViewKind::ApiCatalogView,
            "APICatalog",
            None,
            "API Catalog",
            AutoLayout::TopBottom,
        ));
    }

    model.views = views;
}

fn first_id(model: &Model, kind: ElementKind) -> Option<String> {
    // Stable: keep lexical-id order so repeated runs produce the same view scopes.
    let mut ids: Vec<&str> = model
        .elements
        .values()
        .filter(|e| e.kind == kind)
        .map(|e| e.id.as_str())
        .collect();
    ids.sort();
    ids.first().map(|s| s.to_string())
}

fn make_view(
    kind: ViewKind,
    key: &str,
    scope: Option<String>,
    title: &str,
    layout: AutoLayout,
) -> View {
    View {
        kind,
        key: key.into(),
        scope,
        title: Some(title.into()),
        auto_layout: layout,
        include_all: true,
        animation: Animation::default(),
        composite: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(id: &str, tech: &str) -> Element {
        let mut c = Element::new(id, ElementKind::Container, id);
        c.technology = Some(tech.into());
        c
    }

    #[test]
    fn synthesizes_system_around_orphan_containers() {
        let mut model = Model::default();
        model.name = "example".into();
        model.add_element(container("api", "Rust / Axum"));
        model.add_element(container("db", "PostgreSQL"));

        run(&mut model);

        let systems: Vec<_> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::System)
            .collect();
        assert_eq!(systems.len(), 1);
        let sys = systems[0];
        assert_eq!(sys.children.len(), 2);
        assert!(model
            .elements
            .get("api")
            .unwrap()
            .parent
            .as_deref()
            .is_some());
    }

    #[test]
    fn synthesizes_user_for_web_container() {
        let mut model = Model::default();
        model.add_element(container("api", "Python / Flask"));
        run(&mut model);

        assert!(model
            .elements
            .values()
            .any(|e| e.kind == ElementKind::Person && e.name == "User"));
        assert!(model
            .relationships
            .iter()
            .any(|r| r.label == "uses" && r.to == "api"));
    }

    #[test]
    fn synthesizes_developer_for_pipeline() {
        let mut model = Model::default();
        let mut pipe = Element::new("ci", ElementKind::Pipeline, "CI");
        pipe.tags.push("inferred".into());
        model.add_element(pipe);

        run(&mut model);

        assert!(model
            .elements
            .values()
            .any(|e| e.kind == ElementKind::Person && e.name == "Developer"));
    }

    #[test]
    fn emits_default_views_for_present_kinds() {
        let mut model = Model::default();
        model.name = "example".into();
        model.add_element(container("api", "Go / Gin"));
        let mut p = Element::new("ci", ElementKind::Pipeline, "CI");
        p.tags.push("inferred".into());
        model.add_element(p);

        run(&mut model);

        let kinds: Vec<ViewKind> = model.views.iter().map(|v| v.kind).collect();
        assert!(kinds.contains(&ViewKind::SystemContext));
        assert!(kinds.contains(&ViewKind::Container));
        assert!(kinds.contains(&ViewKind::PipelineView));
        assert!(kinds.contains(&ViewKind::TechStack));
    }

    #[test]
    fn synthesis_is_idempotent() {
        let mut model = Model::default();
        model.name = "example".into();
        model.add_element(container("api", "Rust / Axum"));
        run(&mut model);
        let before = (
            model.elements.len(),
            model.views.len(),
            model.relationships.len(),
            model.tech_stack.len(),
        );
        run(&mut model);
        let after = (
            model.elements.len(),
            model.views.len(),
            model.relationships.len(),
            model.tech_stack.len(),
        );
        assert_eq!(before, after);
    }

    #[test]
    fn does_not_wrap_when_system_already_exists() {
        let mut model = Model::default();
        model.add_element(Element::new("mySys", ElementKind::System, "mySys"));
        let mut c = container("api", "Rust / Axum");
        c.parent = Some("mySys".into());
        model.add_element(c);

        run(&mut model);

        let systems: Vec<_> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::System)
            .collect();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].id, "mySys");
    }

    #[test]
    fn aggregates_tech_stack_from_containers() {
        let mut model = Model::default();
        model.add_element(container("api", "Rust / Axum"));
        model.add_element(container("worker", "Rust / Tokio"));
        model.add_element(container("web", "TypeScript / Next.js"));

        run(&mut model);

        let categories: Vec<&str> = model.tech_stack.iter().map(|c| c.name.as_str()).collect();
        assert!(categories.contains(&"Rust"));
        assert!(categories.contains(&"TypeScript"));
    }
}

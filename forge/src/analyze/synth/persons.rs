//! Synthesises default User and Developer actors when the model has evidence
//! for them — a web-framework container or a CI pipeline, respectively.

use crate::model::*;

use super::{unique_id, SCANNER};
use crate::analyze::provenance::mark_inferred;

/// Add a User and/or Developer actor when the model has evidence for them.
pub(super) fn synthesize_persons(model: &mut Model) {
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

//! Shared rendering helpers: text wrapping, kind labels, CSS slugs, escaping.

use super::TM;
use crate::model::ElementKind;
use crate::text::*;

pub(super) fn render_wrapped_text(
    o: &mut Vec<String>,
    cx: f64,
    mut y: f64,
    max_w: f64,
    text: &str,
    cls: &str,
) {
    let lines = TM.wrap(text, max_w, &FONT_DESC);
    for l in lines.iter().take(3) {
        o.push(format!(
            r#"      <text x="{:.0}" y="{:.0}" class="{}">{}</text>"#,
            cx,
            y,
            cls,
            esc(l)
        ));
        y += FONT_DESC.line_height;
    }
}

pub(super) fn kind_label(kind: ElementKind) -> Option<String> {
    match kind {
        ElementKind::Person => Some("Person".into()),
        ElementKind::System => Some("Software System".into()),
        ElementKind::Container => Some("Container".into()),
        ElementKind::Component => Some("Component".into()),
        ElementKind::Pipeline => Some("Pipeline".into()),
        ElementKind::Repository => Some("Repository".into()),
        ElementKind::DeploymentNode => Some("Deployment Node".into()),
        ElementKind::Branch => Some("Branch".into()),
        ElementKind::Stage | ElementKind::Gate => None,
        _ => None,
    }
}

pub(super) fn css_kind(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Stage => "stage",
        ElementKind::Gate => "gate",
        ElementKind::Pipeline => "pipeline",
        ElementKind::Repository => "repository",
        ElementKind::DeploymentNode => "deploymentnode",
        ElementKind::Branch => "branch",
        _ => "element",
    }
}

pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

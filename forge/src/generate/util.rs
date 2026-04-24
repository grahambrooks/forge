use crate::model::ElementKind;
pub(super) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

pub(super) fn kind_display(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "Person",
        ElementKind::System => "Software System",
        ElementKind::Container => "Container",
        ElementKind::Component => "Component",
        ElementKind::Repository => "Repository",
        ElementKind::Pipeline => "Pipeline",
        ElementKind::Stage => "Stage",
        ElementKind::Gate => "Gate",
        ElementKind::DeploymentNode => "Deployment Node",
        ElementKind::Branch => "Branch",
        _ => "Element",
    }
}

pub(super) fn kind_css(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Person => "person",
        ElementKind::System => "system",
        ElementKind::Container => "container",
        ElementKind::Component => "component",
        ElementKind::Pipeline => "pipeline",
        ElementKind::Stage => "stage",
        ElementKind::Repository => "repository",
        ElementKind::DeploymentNode => "container",
        _ => "container",
    }
}

pub(super) fn kind_order(kind: ElementKind) -> u8 {
    match kind {
        ElementKind::Person => 0,
        ElementKind::System => 1,
        ElementKind::Container => 2,
        ElementKind::Component => 3,
        ElementKind::Pipeline => 4,
        ElementKind::Stage => 5,
        ElementKind::Repository => 6,
        _ => 9,
    }
}

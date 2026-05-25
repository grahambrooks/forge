use crate::model::*;

use super::animate_svg;
use super::derive::derive_dynamic_animation;

#[test]
fn derive_dynamic_animation_generates_cumulative_frames() {
    let mut model = Model::default();
    let mut sys = Element::new("sys", ElementKind::System, "S");
    sys.children = vec!["sys.a".into(), "sys.b".into(), "sys.c".into()];
    model.add_element(sys);
    for id in ["a", "b", "c"] {
        let mut el = Element::new(format!("sys.{id}"), ElementKind::Container, id);
        el.parent = Some("sys".into());
        model.add_element(el);
    }
    model.add_relationship(Relationship {
        frm: "sys.a".into(),
        to: "sys.b".into(),
        label: "1st".into(),
        technology: None,
        order: Some(1),
    });
    model.add_relationship(Relationship {
        frm: "sys.b".into(),
        to: "sys.c".into(),
        label: "2nd".into(),
        technology: None,
        order: Some(2),
    });
    // Unordered relationship — should be ignored by the derivation.
    model.add_relationship(Relationship {
        frm: "sys.a".into(),
        to: "sys.c".into(),
        label: "sidechannel".into(),
        technology: None,
        order: None,
    });

    let view = View {
        kind: ViewKind::Dynamic,
        key: "Flow".into(),
        scope: Some("sys".into()),
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: false,
        animation: Animation::default(),
        composite: None,
    };

    let anim = derive_dynamic_animation(&view, &model).expect("should derive");
    assert_eq!(anim.frames.len(), 2);

    // Frame 1: sys.a + sys.b + the a->b edge
    let f1 = &anim.frames[0];
    assert_eq!(f1.label, "Step 1");
    assert!(f1.includes.contains(&"sys.a".to_string()));
    assert!(f1.includes.contains(&"sys.b".to_string()));
    assert!(f1.includes.contains(&"sys.a -> sys.b".to_string()));
    assert!(!f1.includes.contains(&"sys.c".to_string()));

    // Frame 2: cumulative — everything from frame 1 plus c and b->c
    let f2 = &anim.frames[1];
    assert_eq!(f2.label, "Step 2");
    assert!(f2.includes.contains(&"sys.a".to_string()));
    assert!(f2.includes.contains(&"sys.c".to_string()));
    assert!(f2.includes.contains(&"sys.b -> sys.c".to_string()));
}

#[test]
fn derive_dynamic_animation_ignores_non_dynamic_views() {
    let model = Model::default();
    let view = View {
        kind: ViewKind::Container,
        key: "K".into(),
        scope: None,
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: false,
        animation: Animation::default(),
        composite: None,
    };
    assert!(derive_dynamic_animation(&view, &model).is_none());
}

#[test]
fn derive_dynamic_animation_respects_explicit_animation() {
    let mut model = Model::default();
    let mut sys = Element::new("sys", ElementKind::System, "S");
    sys.children = vec!["sys.a".into()];
    model.add_element(sys);
    let mut a = Element::new("sys.a", ElementKind::Container, "a");
    a.parent = Some("sys".into());
    model.add_element(a);
    model.add_relationship(Relationship {
        frm: "sys.a".into(),
        to: "sys.a".into(),
        label: "self".into(),
        technology: None,
        order: Some(1),
    });

    // User-supplied animation pre-empts the auto-generated one.
    let mut explicit = Animation::default();
    explicit.frames.push(AnimationFrame {
        label: "Handcrafted".into(),
        includes: vec!["sys.a".into()],
        include_all: false,
        highlights: Vec::new(),
        states: Vec::new(),
        notes: None,
    });
    let view = View {
        kind: ViewKind::Dynamic,
        key: "F".into(),
        scope: Some("sys".into()),
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: false,
        animation: explicit,
        composite: None,
    };
    assert!(derive_dynamic_animation(&view, &model).is_none());
}

#[test]
fn empty_animation_passes_through() {
    let svg = "<svg>test</svg>";
    let view = View {
        kind: ViewKind::Container,
        key: "test".into(),
        scope: None,
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: true,
        animation: Animation::default(),
        composite: None,
    };
    let model = Model::default();
    let result = animate_svg(svg, &view, &model);
    assert_eq!(result, svg);
}

#[test]
fn animated_svg_has_frame_classes() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 400" width="800" height="400" class="forge-diagram"><defs><style></style></defs><g class="forge-elements"><g class="forge-element" data-id="svc">content</g></g></svg>"#;

    let mut anim = Animation::default();
    anim.frames.push(AnimationFrame {
        label: "Step 1".into(),
        includes: vec!["svc".into()],
        include_all: false,
        highlights: Vec::new(),
        states: Vec::new(),
        notes: None,
    });

    let view = View {
        kind: ViewKind::Container,
        key: "test".into(),
        scope: None,
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: true,
        animation: anim,
        composite: None,
    };
    let mut model = Model::default();
    model.add_element(Element::new("svc", ElementKind::Container, "Service"));

    let result = animate_svg(svg, &view, &model);
    assert!(result.contains("forge-animated"));
    assert!(result.contains("data-frames=\"1\""));
    assert!(result.contains("forge-frame"));
    assert!(result.contains("forge-frame-dot"));
    assert!(result.contains("forge-enter"));
}

#[test]
fn animation_css_injected() {
    let svg = r#"<svg class="forge-diagram"><defs><style></style></defs></svg>"#;
    let mut anim = Animation::default();
    anim.frames.push(AnimationFrame {
        label: "F1".into(),
        includes: Vec::new(),
        include_all: true,
        highlights: Vec::new(),
        states: Vec::new(),
        notes: None,
    });
    let view = View {
        kind: ViewKind::Container,
        key: "t".into(),
        scope: None,
        title: None,
        auto_layout: AutoLayout::TopBottom,
        include_all: true,
        animation: anim,
        composite: None,
    };
    let result = animate_svg(svg, &view, &Model::default());
    assert!(result.contains("forge-pulse"));
    assert!(result.contains("forge-fade-in"));
}

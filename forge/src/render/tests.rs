use super::util::esc;
use super::{render_svg, render_view as pub_render_view};
use crate::model::ViewKind;
use crate::{layout, parser};

fn render_view(view_key: &str, style: &str) -> String {
    let text = include_str!("../../examples/payments.forge");
    let model = parser::parse(text).unwrap();
    let view = model.views.iter().find(|v| v.key == view_key).unwrap();
    let lo = layout::compute_layout(&model, view);
    render_svg(&lo, style)
}

#[test]
fn svg_is_valid_xml_structure() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.starts_with("<svg "));
    assert!(svg.trim_end().ends_with("</svg>"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
}

#[test]
fn svg_contains_title() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("Payment Platform"));
    assert!(svg.contains("class=\"forge-title\""));
}

#[test]
fn svg_contains_element_groups() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("class=\"forge-elements\""));
    assert!(svg.contains("class=\"forge-relationships\""));
}

#[test]
fn svg_contains_person_element() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("forge-element--person"));
    assert!(svg.contains("Customer"));
    assert!(svg.contains("[Person]"));
}

#[test]
fn svg_contains_system_element() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("forge-element--system"));
    assert!(svg.contains("Payment Service"));
}

#[test]
fn svg_filled_has_drop_shadow() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("id=\"dropShadow\""));
    assert!(svg.contains("filter: url(#dropShadow)"));
}

#[test]
fn svg_outline_disables_shadow() {
    let svg = render_view("SystemContext", "outline");
    assert!(svg.contains("filter: none"));
}

#[test]
fn svg_outline_uses_no_fill() {
    let svg = render_view("SystemContext", "outline");
    assert!(svg.contains("fill: none"));
}

#[test]
fn svg_contains_relationship_labels() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("forge-label--rel"));
}

#[test]
fn data_class_shields_rendered_on_container() {
    let src = r#"
forge "t" {
  model {
    sys = system "Sys" {
      db = container "Ledger" {
        technology "PostgreSQL"
        tags "database"
        data-class "pii" "financial"
      }
    }
  }
  views {
    container-view sys "C" {
      include *
      auto-layout tb
    }
  }
}
"#;
    let model = parser::parse(src).unwrap();
    let view = model.views.first().unwrap();
    let lo = layout::compute_layout(&model, view);
    let svg = render_svg(&lo, "filled");

    assert!(
        svg.contains("forge-dataclass--pii"),
        "expected PII shield CSS class, got:\n{svg}"
    );
    assert!(
        svg.contains("forge-dataclass--financial"),
        "expected financial shield CSS class"
    );
    assert!(svg.contains("#8b5cf6")); // pii purple
    assert!(svg.contains("#d97706")); // financial gold
    assert!(svg.contains("<title>pii</title>"));
    assert!(svg.contains("<title>financial</title>"));
}

#[test]
fn composite_view_assembles_child_svgs() {
    let src = r#"
forge "dash" {
  model {
    sys = system "S" {
      api = container "API"
      db = container "DB"
    }
  }
  views {
    system-context-view sys "Context" {
      include *
    }
    container-view sys "Containers" {
      include *
    }
    composite-view "Dashboard" {
      title "Exec Dashboard"
      grid 2 1
      cell "Context"
      cell "Containers"
    }
  }
}
"#;
    let model = parser::parse(src).unwrap();
    let comp_view = model
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Composite)
        .unwrap();
    let svg = pub_render_view(&model, comp_view, "filled");

    assert!(svg.contains("Exec Dashboard"));
    assert!(svg.contains("forge-composite"));
    assert!(svg.contains(r#"data-view="Context""#));
    assert!(svg.contains(r#"data-view="Containers""#));
    let frame_rects = svg.matches("stroke=\"#d1d5db\"").count();
    assert_eq!(frame_rects, 2);
}

#[test]
fn composite_view_never_recurses_into_composites() {
    let src = r#"
forge "dash" {
  model {
    sys = system "S" { api = container "API" }
  }
  views {
    system-context-view sys "Context" { include * }
    composite-view "Inner" {
      grid 1 1
      cell "Context"
    }
    composite-view "Outer" {
      grid 1 1
      cell "Inner"
    }
  }
}
"#;
    let model = parser::parse(src).unwrap();
    let outer = model.views.iter().find(|v| v.key == "Outer").unwrap();
    let svg = pub_render_view(&model, outer, "filled");
    assert!(svg.contains("forge-composite"));
    assert!(!svg.contains(r#"data-view="Inner""#));
}

#[test]
fn dynamic_view_renders_step_badges() {
    let src = r#"
forge "flow" {
  model {
    user = person "User"
    app = system "App" {
      web = container "Web"
      api = container "API"
      db = container "DB"
    }
  }
  views {
    dynamic-view app "LoginFlow" {
      1. user -> app.web "login" "HTTPS"
      2. app.web -> app.api "POST /login"
      3. app.api -> app.db "SELECT user"
    }
  }
}
"#;
    let model = parser::parse(src).unwrap();
    let view = model.views.first().unwrap();
    let lo = layout::compute_layout(&model, view);
    let svg = render_svg(&lo, "filled");

    assert!(
        svg.contains("forge-step-badge"),
        "expected step badge circle"
    );
    assert!(
        svg.contains("forge-step-label"),
        "expected step badge label"
    );
    for n in ["1", "2", "3"] {
        assert!(
            svg.contains(&format!(">{n}</text>")),
            "expected step number {n} in rendered SVG"
        );
    }
}

#[test]
fn data_class_unknown_falls_back_to_grey() {
    let src = r#"
forge "t" {
  model {
    sys = system "Sys" {
      box = container "Box" {
        data-class "custom-level"
      }
    }
  }
  views {
    container-view sys "C" { include * auto-layout tb }
  }
}
"#;
    let model = parser::parse(src).unwrap();
    let view = model.views.first().unwrap();
    let lo = layout::compute_layout(&model, view);
    let svg = render_svg(&lo, "filled");
    assert!(svg.contains("forge-dataclass--custom-level"));
    assert!(svg.contains("#6b7280"));
}

#[test]
fn svg_containers_has_database_cylinder() {
    let svg = render_view("Containers", "filled");
    assert!(svg.contains("forge-element--database"));
    assert!(svg.contains("ellipse"));
    assert!(svg.contains("[Database]"));
}

#[test]
fn svg_containers_outline_cylinder() {
    let svg = render_view("Containers", "outline");
    assert!(svg.contains("forge-element--database"));
    assert!(svg.contains("<path d=\"M"));
}

#[test]
fn svg_containers_has_technology_labels() {
    let svg = render_view("Containers", "filled");
    assert!(svg.contains("[Rust / Actix]"));
    assert!(svg.contains("[PostgreSQL 16]"));
    assert!(svg.contains("[Redis]"));
}

#[test]
fn svg_pipeline_has_stages() {
    let svg = render_view("Pipeline", "filled");
    assert!(svg.contains("forge-element--stage"));
    assert!(svg.contains("Build &amp; Test"));
    assert!(svg.contains("Security Scan"));
    assert!(svg.contains("Deploy Staging"));
    assert!(svg.contains("Deploy Production"));
}

#[test]
fn svg_pipeline_has_gates() {
    let svg = render_view("Pipeline", "filled");
    assert!(svg.contains("forge-element--gate"));
    assert!(svg.contains("<polygon"));
}

#[test]
fn svg_pipeline_has_connectors() {
    let svg = render_view("Pipeline", "filled");
    assert!(svg.contains("forge-connector"));
    assert!(svg.contains("arrow-pipe"));
}

#[test]
fn svg_contains_legend() {
    let svg = render_view("SystemContext", "filled");
    assert!(svg.contains("forge-legend"));
    assert!(svg.contains("Legend"));
}

#[test]
fn svg_legend_outline_swatches() {
    let svg = render_view("SystemContext", "outline");
    assert!(svg.contains("forge-legend-swatch"));
    assert!(svg.contains(r#"fill="none""#));
}

#[test]
fn svg_has_white_background() {
    let svg = render_view("Containers", "filled");
    assert!(svg.contains("forge-bg"));
}

#[test]
fn svg_has_arrowhead_markers() {
    let svg = render_view("Containers", "filled");
    assert!(svg.contains("id=\"arrow\""));
    assert!(svg.contains("marker-end="));
}

#[test]
fn esc_html_entities() {
    assert_eq!(esc("a<b>c&d\"e"), "a&lt;b&gt;c&amp;d&quot;e");
}

#[test]
fn all_views_render_both_styles() {
    for key in &["SystemContext", "Containers", "Pipeline", "Deployment"] {
        for style in &["filled", "outline"] {
            let svg = render_view(key, style);
            assert!(svg.contains("<svg "), "{key}/{style} missing svg tag");
            assert!(svg.contains("</svg>"), "{key}/{style} missing closing svg");
        }
    }
}

#[test]
fn svg_deployment_has_nested_nodes() {
    let svg = render_view("Deployment", "filled");
    assert!(svg.contains("forge-element--deploymentnode"));
    assert!(svg.contains("EKS Cluster"));
    assert!(svg.contains("stroke-dasharray"));
}

#[test]
fn svg_deployment_has_container_instances() {
    let svg = render_view("Deployment", "filled");
    assert!(svg.contains("Payment API") || svg.contains("Ledger DB"));
}

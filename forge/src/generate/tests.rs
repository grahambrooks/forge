use super::*;
use crate::parser;

fn payments_model() -> Model {
    let text = include_str!("../../examples/payments.forge");
    parser::parse(text).unwrap()
}

#[test]
fn generate_produces_site() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test");
    let _ = fs::remove_dir_all(&tmp);

    let config = GenerateConfig {
        out_dir: tmp.clone(),
        ..Default::default()
    };

    let report = generate(&model, &config, None).expect("generate should succeed");
    assert!(report.pages > 0);
    assert_eq!(report.diagrams, 13);

    // Check files exist
    assert!(tmp.join("index.html").exists());
    assert!(tmp.join("assets/forge.css").exists());
    assert!(tmp.join("forge.json").exists());
    assert!(tmp.join("views/SystemContext.html").exists());
    assert!(tmp.join("views/Containers.html").exists());
    assert!(tmp.join("views/Pipeline.html").exists());
    assert!(tmp.join("views/Deployment.html").exists());
    assert!(tmp.join("assets/diagrams/SystemContext.svg").exists());
    assert!(tmp.join("assets/diagrams/Deployment.svg").exists());

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn index_contains_model_info() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test_idx");
    let _ = fs::remove_dir_all(&tmp);

    let config = GenerateConfig {
        out_dir: tmp.clone(),
        ..Default::default()
    };
    generate(&model, &config, None).unwrap();

    let html = fs::read_to_string(tmp.join("index.html")).unwrap();
    assert!(html.contains("Payment Platform"));
    assert!(html.contains("Actors"));
    assert!(html.contains("Containers"));
    assert!(html.contains("forge.css"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn view_page_contains_svg() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test_view");
    let _ = fs::remove_dir_all(&tmp);

    let config = GenerateConfig {
        out_dir: tmp.clone(),
        ..Default::default()
    };
    generate(&model, &config, None).unwrap();

    let html = fs::read_to_string(tmp.join("views/SystemContext.html")).unwrap();
    assert!(html.contains("<svg"));
    assert!(html.contains("forge-diagram-wrap"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn element_page_has_properties() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test_el");
    let _ = fs::remove_dir_all(&tmp);

    let config = GenerateConfig {
        out_dir: tmp.clone(),
        ..Default::default()
    };
    generate(&model, &config, None).unwrap();

    let html = fs::read_to_string(tmp.join("elements/payments-api.html")).unwrap();
    assert!(html.contains("Payment API"));
    assert!(html.contains("Rust / Actix"));
    assert!(html.contains("Container"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn json_export_is_valid() {
    let model = payments_model();
    let json = render_json(&model);
    assert!(json.contains("\"name\": \"Payment Platform\""));
    assert!(json.contains("\"elements\""));
    assert!(json.contains("\"relationships\""));
}

#[test]
fn nav_has_links() {
    let model = payments_model();
    let nav = build_nav(&model, "/");
    assert!(nav.contains("Payment Platform"));
    assert!(nav.contains("views/SystemContext.html"));
    assert!(nav.contains("elements/"));
}

#[test]
fn nav_has_docs_section() {
    let model = payments_model();
    let nav = build_nav(&model, "/");
    assert!(nav.contains("Documentation"));
    assert!(nav.contains("docs/overview.html"));
    assert!(nav.contains("docs/security.html"));
}

fn examples_config(tmp: PathBuf) -> GenerateConfig {
    // Point source_dir at the examples/ directory so doc files resolve
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    GenerateConfig {
        out_dir: tmp,
        source_dir: examples_dir,
        ..Default::default()
    }
}

#[test]
fn generate_produces_doc_pages() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test_docs");
    let _ = fs::remove_dir_all(&tmp);

    let config = examples_config(tmp.clone());
    let report = generate(&model, &config, None).expect("generate should succeed");
    assert!(report.pages >= 21); // 17 + 4 docs

    assert!(tmp.join("docs/overview.html").exists());
    assert!(tmp.join("docs/architecture-decisions.html").exists());
    assert!(tmp.join("docs/deployment.html").exists());
    assert!(tmp.join("docs/security.html").exists());

    let html = fs::read_to_string(tmp.join("docs/overview.html")).unwrap();
    assert!(html.contains("Overview"));
    assert!(html.contains("Business Context"));
    assert!(html.contains("forge-doc"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn doc_page_renders_markdown() {
    let model = payments_model();
    let tmp = std::env::temp_dir().join("forge_gen_test_docs_md");
    let _ = fs::remove_dir_all(&tmp);

    let config = examples_config(tmp.clone());
    generate(&model, &config, None).unwrap();

    let html = fs::read_to_string(tmp.join("docs/architecture-decisions.html")).unwrap();
    // Markdown headings should render as HTML
    assert!(html.contains("<h2>"));
    // Bold text from **Status**: etc
    assert!(html.contains("<strong>"));
    // List items from consequences
    assert!(html.contains("<li>"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn markdown_renderer_basic() {
    let html = render_markdown("# Hello\n\nSome **bold** text.\n\n- item 1\n- item 2\n");
    assert!(html.contains("<h1>Hello</h1>"));
    assert!(html.contains("<strong>bold</strong>"));
    assert!(html.contains("<li>item 1</li>"));
}

fn baseline_model() -> Model {
    let text = include_str!("../../examples/payments-baseline.forge");
    parser::parse(text).unwrap()
}

#[test]
fn generate_with_diff_shows_changes() {
    let baseline = baseline_model();
    let current = payments_model();
    let dr = crate::diff::diff(&baseline, &current);

    let tmp = std::env::temp_dir().join("forge_gen_test_diff");
    let _ = fs::remove_dir_all(&tmp);
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let config = GenerateConfig {
        out_dir: tmp.clone(),
        source_dir: examples_dir,
        ..Default::default()
    };

    generate(&current, &config, Some(&dr)).unwrap();

    // Index should have diff banner
    let index = fs::read_to_string(tmp.join("index.html")).unwrap();
    assert!(index.contains("What Changed"));
    assert!(index.contains("forge-diff-banner"));
    assert!(index.contains("Added"));

    // SVGs should have diff highlight classes
    let svg = fs::read_to_string(tmp.join("assets/diagrams/Containers.svg")).unwrap();
    assert!(
        svg.contains("forge-diff-highlight--added"),
        "SVG should highlight added elements"
    );

    // New ADR should be linked as rationale
    assert!(index.contains("ADR-006"));

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn diff_highlights_injected_into_svg() {
    let svg =
        r#"<svg><defs><style></style></defs><g class="forge-element" data-id="new.svc"></g></svg>"#;
    let mut dr = DiffResult {
        element_changes: Vec::new(),
        relationship_changes: Vec::new(),
        doc_changes: Vec::new(),
        description: String::new(),
        added_ids: std::collections::HashSet::new(),
        modified_ids: std::collections::HashSet::new(),
        removed_ids: std::collections::HashSet::new(),
    };
    dr.added_ids.insert("new.svc".into());

    let result = inject_diff_highlights(svg, &dr);
    assert!(result.contains("forge-diff-highlight--added"));
    assert!(result.contains("stroke: #16a34a"));
}

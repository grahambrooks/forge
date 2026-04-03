//! Forge site generator — produces a static documentation website from a model.
//!
//! Generated site structure:
//!   _site/
//!   ├── index.html           — Landing page with model overview
//!   ├── views/*.html         — One page per view with embedded SVG
//!   ├── elements/*.html      — Detail page per element
//!   ├── docs/*.html          — Markdown documentation pages
//!   ├── assets/
//!   │   ├── forge.css        — Default site styles
//!   │   └── diagrams/*.svg   — Rendered SVGs
//!   └── forge.json           — Machine-readable model export

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::check;
use crate::diff::{ChangeKind, DiffResult};
use crate::layout;
use crate::model::*;
use crate::render;

/// Configuration for site generation.
pub struct GenerateConfig {
    pub out_dir: PathBuf,
    pub title: Option<String>,
    pub base_url: String,
    pub style: String,
    pub source_dir: PathBuf,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("_site"),
            title: None,
            base_url: "/".into(),
            style: "filled".into(),
            source_dir: PathBuf::from("."),
        }
    }
}

/// Generate the complete static site.
pub fn generate(
    model: &Model,
    config: &GenerateConfig,
    diff: Option<&DiffResult>,
) -> Result<GenerateReport, String> {
    let out = &config.out_dir;
    let title = config
        .title
        .as_deref()
        .unwrap_or_else(|| {
            if model.name.is_empty() {
                "Architecture"
            } else {
                &model.name
            }
        })
        .to_string();
    let base = &config.base_url;

    // Create directories
    for dir in &["", "views", "elements", "docs", "assets", "assets/diagrams"] {
        fs::create_dir_all(out.join(dir)).map_err(|e| format!("mkdir: {}", e))?;
    }

    // Render all view SVGs (with diff highlighting if available)
    let mut view_svgs: HashMap<String, String> = HashMap::new();
    for view in &model.views {
        let lo = layout::compute_layout(model, view);
        let mut svg = render::render_svg(&lo, &config.style);
        if let Some(dr) = diff {
            svg = inject_diff_highlights(&svg, dr);
        }
        let svg_path = out.join(format!("assets/diagrams/{}.svg", view.key));
        fs::write(&svg_path, &svg).map_err(|e| format!("write svg: {}", e))?;
        view_svgs.insert(view.key.clone(), svg);
    }

    // Run checks for summary
    let violations = check::check(model, check::Severity::Info);

    // Collect structural elements sorted by kind
    let mut elements: Vec<&Element> = model.elements.values().collect();
    elements.sort_by(|a, b| {
        kind_order(a.kind)
            .cmp(&kind_order(b.kind))
            .then(a.name.cmp(&b.name))
    });

    // Generate pages — root pages use "./" prefix, subdir pages use "../"
    let root_prefix = if base == "/" { "./" } else { base.as_str() };
    let sub_prefix = if base == "/" { "../" } else { base.as_str() };

    // Index page
    let nav_root = build_nav(model, root_prefix);
    let index_html = render_index(&title, model, &violations, &nav_root, root_prefix, diff);
    fs::write(out.join("index.html"), &index_html).map_err(|e| format!("write: {}", e))?;

    // View pages
    let nav_sub = build_nav(model, sub_prefix);
    for view in &model.views {
        let svg = view_svgs.get(&view.key).unwrap();
        let html = render_view_page(&title, model, view, svg, &nav_sub, sub_prefix, diff);
        fs::write(out.join(format!("views/{}.html", view.key)), &html)
            .map_err(|e| format!("write: {}", e))?;
    }

    // Element pages
    for el in &elements {
        if matches!(
            el.kind,
            ElementKind::Gate | ElementKind::Step | ElementKind::Artifact
        ) {
            continue;
        }
        let html = render_element_page(&title, model, el, &nav_sub, sub_prefix, diff);
        let slug = el.id.replace('.', "-");
        fs::write(out.join(format!("elements/{}.html", slug)), &html)
            .map_err(|e| format!("write: {}", e))?;
    }

    // Doc pages
    let mut doc_count = 0;
    for doc in &model.docs {
        let md_path = config.source_dir.join(&doc.path);
        let md_content = match fs::read_to_string(&md_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  Warning: could not read {}: {}", md_path.display(), e);
                continue;
            }
        };
        let html_body = render_markdown(&md_content);
        let html = render_doc_page(&title, doc, &html_body, &nav_sub, sub_prefix);
        let slug = slugify_doc(&doc.title);
        fs::write(out.join(format!("docs/{}.html", slug)), &html)
            .map_err(|e| format!("write: {}", e))?;
        doc_count += 1;
    }

    // CSS
    fs::write(out.join("assets/forge.css"), CSS).map_err(|e| format!("write css: {}", e))?;

    // JSON export
    let json = render_json(model);
    fs::write(out.join("forge.json"), &json).map_err(|e| format!("write json: {}", e))?;

    let page_count = 1
        + model.views.len()
        + elements
            .iter()
            .filter(|e| {
                !matches!(
                    e.kind,
                    ElementKind::Gate | ElementKind::Step | ElementKind::Artifact
                )
            })
            .count()
        + doc_count;
    Ok(GenerateReport {
        pages: page_count,
        diagrams: model.views.len(),
    })
}

pub struct GenerateReport {
    pub pages: usize,
    pub diagrams: usize,
}

// ─── Navigation ──────────────────────────────────────────────────

fn build_nav(model: &Model, base: &str) -> String {
    let mut nav = String::new();
    nav.push_str(&format!(
        r#"<nav class="forge-nav"><a href="{}index.html" class="forge-nav__home">{}</a>"#,
        base,
        esc(&model.name)
    ));
    nav.push_str(r#"<div class="forge-nav__section"><span>Views</span><ul>"#);
    for view in &model.views {
        nav.push_str(&format!(
            r#"<li><a href="{}views/{}.html">{}</a></li>"#,
            base,
            view.key,
            esc(view.title.as_deref().unwrap_or(&view.key))
        ));
    }
    nav.push_str("</ul></div>");
    nav.push_str(r#"<div class="forge-nav__section"><span>Elements</span><ul>"#);
    let mut els: Vec<&Element> = model
        .elements
        .values()
        .filter(|e| {
            matches!(
                e.kind,
                ElementKind::Person
                    | ElementKind::System
                    | ElementKind::Container
                    | ElementKind::Component
                    | ElementKind::Pipeline
            )
        })
        .collect();
    els.sort_by(|a, b| a.name.cmp(&b.name));
    for el in &els {
        let slug = el.id.replace('.', "-");
        nav.push_str(&format!(
            r#"<li><a href="{}elements/{}.html">{}</a></li>"#,
            base,
            slug,
            esc(&el.name)
        ));
    }
    nav.push_str("</ul></div>");
    if !model.docs.is_empty() {
        nav.push_str(r#"<div class="forge-nav__section"><span>Documentation</span><ul>"#);
        for doc in &model.docs {
            let slug = slugify_doc(&doc.title);
            nav.push_str(&format!(
                r#"<li><a href="{}docs/{}.html">{}</a></li>"#,
                base,
                slug,
                esc(&doc.title)
            ));
        }
        nav.push_str("</ul></div>");
    }
    nav.push_str("</nav>");
    nav
}

// ─── Index Page ──────────────────────────────────────────────────

fn render_index(
    title: &str,
    model: &Model,
    violations: &[check::Violation],
    nav: &str,
    base: &str,
    diff: Option<&DiffResult>,
) -> String {
    let mut main = String::new();

    main.push_str(&format!("<h1>{}</h1>", esc(title)));

    // Diff summary banner
    if let Some(dr) = diff {
        main.push_str(r#"<div class="forge-diff-banner">"#);
        main.push_str(r#"<h2>What Changed</h2>"#);
        main.push_str(&format!(
            r#"<p class="forge-diff-desc">{}</p>"#,
            esc(&dr.description)
        ));
        main.push_str(&format!(
            r#"<div class="forge-diff-stats"><span class="forge-diff--added">{} added</span> <span class="forge-diff--modified">{} modified</span> <span class="forge-diff--removed">{} removed</span></div>"#,
            dr.added_count(),
            dr.modified_count(),
            dr.removed_count()
        ));

        // Detailed change table
        if !dr.element_changes.is_empty() {
            main.push_str(r#"<table class="forge-table"><thead><tr><th>Change</th><th>Element</th><th>Type</th><th>Details</th></tr></thead><tbody>"#);
            for c in &dr.element_changes {
                let (cls, label) = match c.change {
                    ChangeKind::Added => ("forge-diff--added", "Added"),
                    ChangeKind::Modified => ("forge-diff--modified", "Modified"),
                    ChangeKind::Removed => ("forge-diff--removed", "Removed"),
                };
                let slug = c.id.replace('.', "-");
                let el_link = if c.change != ChangeKind::Removed {
                    format!(
                        r#"<a href="{}elements/{}.html">{}</a>"#,
                        base,
                        slug,
                        esc(&c.name)
                    )
                } else {
                    esc(&c.name)
                };
                main.push_str(&format!(
                    r#"<tr><td class="{}">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
                    cls,
                    label,
                    el_link,
                    kind_display(c.kind),
                    esc(&c.details.join("; "))
                ));
            }
            main.push_str("</tbody></table>");
        }

        // Link to driving ADR (new docs)
        let new_docs: Vec<_> = dr
            .doc_changes
            .iter()
            .filter(|d| d.change == ChangeKind::Added)
            .collect();
        if !new_docs.is_empty() {
            main.push_str(r#"<div class="forge-diff-rationale"><strong>Rationale:</strong> "#);
            for (i, d) in new_docs.iter().enumerate() {
                let slug = slugify_doc(&d.title);
                if i > 0 {
                    main.push_str(", ");
                }
                main.push_str(&format!(
                    r#"<a href="{}docs/{}.html">{}</a>"#,
                    base,
                    slug,
                    esc(&d.title)
                ));
            }
            main.push_str("</div>");
        }

        main.push_str("</div>");
    }
    if !model.description.is_empty() {
        main.push_str(&format!(
            "<p class=\"forge-desc\">{}</p>",
            esc(&model.description)
        ));
    }

    // Stats
    let persons = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Person)
        .count();
    let systems = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::System)
        .count();
    let containers = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Container)
        .count();
    let pipelines = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Pipeline)
        .count();

    main.push_str(r#"<div class="forge-stats">"#);
    if persons > 0 {
        main.push_str(&format!(r#"<div class="forge-stat"><span class="forge-stat__num">{}</span><span>Actors</span></div>"#, persons));
    }
    if systems > 0 {
        main.push_str(&format!(r#"<div class="forge-stat"><span class="forge-stat__num">{}</span><span>Systems</span></div>"#, systems));
    }
    if containers > 0 {
        main.push_str(&format!(r#"<div class="forge-stat"><span class="forge-stat__num">{}</span><span>Containers</span></div>"#, containers));
    }
    if pipelines > 0 {
        main.push_str(&format!(r#"<div class="forge-stat"><span class="forge-stat__num">{}</span><span>Pipelines</span></div>"#, pipelines));
    }
    main.push_str(&format!(
        r#"<div class="forge-stat"><span class="forge-stat__num">{}</span><span>Relationships</span></div>"#,
        model.relationships.len()
    ));
    main.push_str("</div>");

    // Views
    if !model.views.is_empty() {
        main.push_str("<h2>Views</h2><div class=\"forge-cards\">");
        for view in &model.views {
            let view_title = view.title.as_deref().unwrap_or(&view.key);
            main.push_str(&format!(
                r#"<a class="forge-card" href="{}views/{}.html"><div class="forge-card__title">{}</div><div class="forge-card__sub">{:?} view</div></a>"#,
                base, view.key, esc(view_title), view.kind
            ));
        }
        main.push_str("</div>");
    }

    // Check results
    if !violations.is_empty() {
        let errors = violations
            .iter()
            .filter(|v| v.severity == check::Severity::Error)
            .count();
        let warnings = violations
            .iter()
            .filter(|v| v.severity == check::Severity::Warning)
            .count();
        let infos = violations
            .iter()
            .filter(|v| v.severity == check::Severity::Info)
            .count();

        main.push_str("<h2>Architecture Checks</h2>");
        main.push_str(&format!(
            r#"<div class="forge-checks-summary">{} error(s), {} warning(s), {} info(s)</div>"#,
            errors, warnings, infos
        ));
        main.push_str(r#"<table class="forge-table"><thead><tr><th>Severity</th><th>Rule</th><th>Element</th><th>Message</th></tr></thead><tbody>"#);
        for v in violations {
            let sev_class = match v.severity {
                check::Severity::Error => "forge-sev--error",
                check::Severity::Warning => "forge-sev--warning",
                check::Severity::Info => "forge-sev--info",
            };
            let el_link = if let Some(ref id) = v.element_id {
                let slug = id.replace('.', "-");
                format!(
                    r#"<a href="{}elements/{}.html">{}</a>"#,
                    base,
                    slug,
                    esc(id)
                )
            } else {
                "-".into()
            };
            main.push_str(&format!(
                r#"<tr><td class="{}">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
                sev_class,
                v.severity,
                v.rule,
                el_link,
                esc(&v.message)
            ));
        }
        main.push_str("</tbody></table>");
    }

    page_template(title, &main, nav, base)
}

// ─── View Page ───────────────────────────────────────────────────

fn render_view_page(
    title: &str,
    _model: &Model,
    view: &View,
    svg: &str,
    nav: &str,
    base: &str,
    diff: Option<&DiffResult>,
) -> String {
    let view_title = view.title.as_deref().unwrap_or(&view.key);
    let mut main = String::new();

    main.push_str(&format!("<h1>{}</h1>", esc(view_title)));

    // Diff legend if changes exist
    if let Some(dr) = diff {
        if !dr.is_empty() {
            main.push_str(r#"<div class="forge-diff-legend">"#);
            main.push_str(r#"<span class="forge-diff--added">&#9632; Added</span> "#);
            main.push_str(r#"<span class="forge-diff--modified">&#9632; Modified</span> "#);
            main.push_str(r#"<span class="forge-diff--removed">&#9632; Removed</span>"#);
            main.push_str("</div>");
        }
    }

    main.push_str(&format!(r#"<div class="forge-diagram-wrap">{}</div>"#, svg));

    page_template(&format!("{} — {}", view_title, title), &main, nav, base)
}

// ─── Element Page ────────────────────────────────────────────────

fn render_element_page(
    title: &str,
    model: &Model,
    el: &Element,
    nav: &str,
    base: &str,
    diff: Option<&DiffResult>,
) -> String {
    let mut main = String::new();

    main.push_str(&format!("<h1>{}</h1>", esc(&el.name)));
    main.push_str(&format!(
        r#"<span class="forge-badge forge-badge--{}">{}</span>"#,
        kind_css(el.kind),
        kind_display(el.kind)
    ));

    // Diff change indicator
    if let Some(dr) = diff {
        if dr.added_ids.contains(&el.id) {
            main.push_str(r#" <span class="forge-badge forge-diff--added">Added</span>"#);
        } else if dr.modified_ids.contains(&el.id) {
            main.push_str(r#" <span class="forge-badge forge-diff--modified">Modified</span>"#);
        }
    }

    // Properties table
    main.push_str(r#"<table class="forge-table forge-props"><tbody>"#);
    main.push_str(&format!(
        "<tr><th>ID</th><td><code>{}</code></td></tr>",
        esc(&el.id)
    ));
    if let Some(ref desc) = el.description {
        main.push_str(&format!(
            "<tr><th>Description</th><td>{}</td></tr>",
            esc(desc)
        ));
    }
    if let Some(ref tech) = el.technology {
        main.push_str(&format!(
            "<tr><th>Technology</th><td>{}</td></tr>",
            esc(tech)
        ));
    }
    if !el.tags.is_empty() {
        let tags_html: Vec<String> = el
            .tags
            .iter()
            .map(|t| format!(r#"<span class="forge-tag">{}</span>"#, esc(t)))
            .collect();
        main.push_str(&format!(
            "<tr><th>Tags</th><td>{}</td></tr>",
            tags_html.join(" ")
        ));
    }
    if let Some(ref parent_id) = el.parent {
        let slug = parent_id.replace('.', "-");
        let parent_name = model
            .elements
            .get(parent_id)
            .map(|e| e.name.as_str())
            .unwrap_or(parent_id);
        main.push_str(&format!(
            r#"<tr><th>Parent</th><td><a href="{}elements/{}.html">{}</a></td></tr>"#,
            base,
            slug,
            esc(parent_name)
        ));
    }
    for (k, v) in &el.properties {
        main.push_str(&format!("<tr><th>{}</th><td>{}</td></tr>", esc(k), esc(v)));
    }
    main.push_str("</tbody></table>");

    // Children
    if !el.children.is_empty() {
        main.push_str("<h2>Children</h2><ul>");
        for child_id in &el.children {
            if let Some(child) = model.elements.get(child_id) {
                let slug = child_id.replace('.', "-");
                main.push_str(&format!(
                    r#"<li><a href="{}elements/{}.html">{}</a> <span class="forge-badge forge-badge--{}">{}</span></li>"#,
                    base, slug, esc(&child.name), kind_css(child.kind), kind_display(child.kind)
                ));
            }
        }
        main.push_str("</ul>");
    }

    // Relationships
    let outgoing: Vec<&Relationship> = model
        .relationships
        .iter()
        .filter(|r| r.frm == el.id)
        .collect();
    let incoming: Vec<&Relationship> = model
        .relationships
        .iter()
        .filter(|r| r.to == el.id)
        .collect();

    if !outgoing.is_empty() {
        main.push_str("<h2>Outgoing Relationships</h2>");
        main.push_str(r#"<table class="forge-table"><thead><tr><th>Target</th><th>Label</th><th>Technology</th></tr></thead><tbody>"#);
        for r in &outgoing {
            let target_name = model
                .elements
                .get(&r.to)
                .map(|e| e.name.as_str())
                .unwrap_or(&r.to);
            let slug = r.to.replace('.', "-");
            main.push_str(&format!(
                r#"<tr><td><a href="{}elements/{}.html">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                base,
                slug,
                esc(target_name),
                esc(&r.label),
                esc(r.technology.as_deref().unwrap_or("-"))
            ));
        }
        main.push_str("</tbody></table>");
    }

    if !incoming.is_empty() {
        main.push_str("<h2>Incoming Relationships</h2>");
        main.push_str(r#"<table class="forge-table"><thead><tr><th>Source</th><th>Label</th><th>Technology</th></tr></thead><tbody>"#);
        for r in &incoming {
            let source_name = model
                .elements
                .get(&r.frm)
                .map(|e| e.name.as_str())
                .unwrap_or(&r.frm);
            let slug = r.frm.replace('.', "-");
            main.push_str(&format!(
                r#"<tr><td><a href="{}elements/{}.html">{}</a></td><td>{}</td><td>{}</td></tr>"#,
                base,
                slug,
                esc(source_name),
                esc(&r.label),
                esc(r.technology.as_deref().unwrap_or("-"))
            ));
        }
        main.push_str("</tbody></table>");
    }

    page_template(&format!("{} — {}", el.name, title), &main, nav, base)
}

// ─── Doc Page ────────────────────────────────────────────────────

fn render_doc_page(
    title: &str,
    doc: &crate::model::Doc,
    html_body: &str,
    nav: &str,
    base: &str,
) -> String {
    let mut main = String::new();
    main.push_str(&format!("<h1>{}</h1>", esc(&doc.title)));
    main.push_str(&format!(r#"<div class="forge-doc">{}</div>"#, html_body));
    page_template(&format!("{} — {}", doc.title, title), &main, nav, base)
}

fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let parser = Parser::new_ext(md, opts);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn slugify_doc(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_string()
}

// ─── JSON Export ─────────────────────────────────────────────────

fn render_json(model: &Model) -> String {
    let mut o = String::from("{\n");
    o.push_str(&format!("  \"name\": \"{}\",\n", json_esc(&model.name)));
    o.push_str(&format!(
        "  \"description\": \"{}\",\n",
        json_esc(&model.description)
    ));

    // Elements
    o.push_str("  \"elements\": [\n");
    let els: Vec<&Element> = model.elements.values().collect();
    for (i, el) in els.iter().enumerate() {
        let comma = if i + 1 < els.len() { "," } else { "" };
        let tags = el
            .tags
            .iter()
            .map(|t| format!("\"{}\"", json_esc(t)))
            .collect::<Vec<_>>()
            .join(", ");
        o.push_str(&format!(
            "    {{\"id\": \"{}\", \"kind\": \"{}\", \"name\": \"{}\", \"description\": {}, \"technology\": {}, \"tags\": [{}], \"parent\": {}}}{}\n",
            json_esc(&el.id),
            kind_display(el.kind),
            json_esc(&el.name),
            el.description.as_ref().map(|d| format!("\"{}\"", json_esc(d))).unwrap_or_else(|| "null".into()),
            el.technology.as_ref().map(|t| format!("\"{}\"", json_esc(t))).unwrap_or_else(|| "null".into()),
            tags,
            el.parent.as_ref().map(|p| format!("\"{}\"", json_esc(p))).unwrap_or_else(|| "null".into()),
            comma
        ));
    }
    o.push_str("  ],\n");

    // Relationships
    o.push_str("  \"relationships\": [\n");
    for (i, r) in model.relationships.iter().enumerate() {
        let comma = if i + 1 < model.relationships.len() {
            ","
        } else {
            ""
        };
        o.push_str(&format!(
            "    {{\"from\": \"{}\", \"to\": \"{}\", \"label\": \"{}\", \"technology\": {}}}{}\n",
            json_esc(&r.frm),
            json_esc(&r.to),
            json_esc(&r.label),
            r.technology
                .as_ref()
                .map(|t| format!("\"{}\"", json_esc(t)))
                .unwrap_or_else(|| "null".into()),
            comma
        ));
    }
    o.push_str("  ]\n}\n");
    o
}

// ─── HTML Template ───────────────────────────────────────────────

fn page_template(title: &str, main_content: &str, nav: &str, base: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="{base}assets/forge.css">
</head>
<body>
<div class="forge-layout">
{nav}
<main class="forge-main">
{main}
</main>
</div>
</body>
</html>
"#,
        title = esc(title),
        base = base,
        nav = nav,
        main = main_content,
    )
}

// ─── CSS Theme ───────────────────────────────────────────────────

const CSS: &str = r#"/* Forge documentation site — default theme */
*,*::before,*::after{box-sizing:border-box}
body{margin:0;font-family:system-ui,-apple-system,'Segoe UI',Helvetica,Arial,sans-serif;color:#1a1a2e;background:#fafbfc;line-height:1.6}
a{color:#1168BD;text-decoration:none}
a:hover{text-decoration:underline}
code{background:#f0f2f5;padding:2px 6px;border-radius:3px;font-size:0.9em}
h1{margin:0 0 0.5em;font-size:1.75rem;font-weight:700}
h2{margin:1.5em 0 0.5em;font-size:1.25rem;font-weight:600;border-bottom:1px solid #e2e6ea;padding-bottom:0.3em}

/* Layout */
.forge-layout{display:flex;min-height:100vh}
.forge-nav{width:260px;background:#1a1a2e;color:#c8cdd3;padding:1rem 0;flex-shrink:0;font-size:0.875rem;overflow-y:auto;position:sticky;top:0;height:100vh}
.forge-nav a{color:#c8cdd3;display:block;padding:4px 1.25rem}
.forge-nav a:hover{color:#fff;background:rgba(255,255,255,0.06);text-decoration:none}
.forge-nav__home{font-weight:700;font-size:1rem;color:#fff!important;padding:0.5rem 1.25rem 1rem!important;border-bottom:1px solid rgba(255,255,255,0.08);margin-bottom:0.5rem}
.forge-nav__section{margin:0.5rem 0}
.forge-nav__section>span{display:block;padding:0.5rem 1.25rem 0.2rem;font-size:0.7rem;text-transform:uppercase;letter-spacing:0.05em;color:#8890a0;font-weight:600}
.forge-nav ul{list-style:none;margin:0;padding:0}
.forge-nav li a{padding-left:2rem}
.forge-main{flex:1;padding:2rem 3rem;max-width:1100px}

/* Description */
.forge-desc{font-size:1.05rem;color:#555;margin-bottom:1.5em}

/* Stats row */
.forge-stats{display:flex;gap:1.5rem;margin:1.5em 0;flex-wrap:wrap}
.forge-stat{background:#fff;border:1px solid #e2e6ea;border-radius:8px;padding:1rem 1.5rem;text-align:center;min-width:100px}
.forge-stat__num{display:block;font-size:1.75rem;font-weight:700;color:#1168BD}

/* Cards */
.forge-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:1rem;margin:1em 0}
.forge-card{display:block;background:#fff;border:1px solid #e2e6ea;border-radius:8px;padding:1.25rem;transition:box-shadow .15s}
.forge-card:hover{box-shadow:0 2px 8px rgba(0,0,0,0.08);text-decoration:none}
.forge-card__title{font-weight:600;font-size:1rem;margin-bottom:0.3em}
.forge-card__sub{font-size:0.85rem;color:#888}

/* Tables */
.forge-table{width:100%;border-collapse:collapse;margin:0.5em 0 1.5em;font-size:0.9rem}
.forge-table th,.forge-table td{text-align:left;padding:0.5rem 0.75rem;border-bottom:1px solid #e2e6ea}
.forge-table thead th{background:#f6f8fa;font-weight:600;font-size:0.8rem;text-transform:uppercase;letter-spacing:0.03em;color:#555}
.forge-props th{width:140px;color:#555;font-weight:500;background:#f9fafb}

/* Badges */
.forge-badge{display:inline-block;font-size:0.7rem;font-weight:600;padding:2px 8px;border-radius:10px;text-transform:uppercase;letter-spacing:0.03em;vertical-align:middle}
.forge-badge--person{background:#e8f0fe;color:#08427B}
.forge-badge--system{background:#dbeafe;color:#1168BD}
.forge-badge--container{background:#e0f0ff;color:#438DD5}
.forge-badge--component{background:#f0f7ff;color:#6BA3D6}
.forge-badge--pipeline{background:#f3e8ff;color:#7c3aed}
.forge-badge--stage{background:#f5f5f5;color:#555}
.forge-badge--repository{background:#ecfdf5;color:#059669}

/* Tags */
.forge-tag{display:inline-block;font-size:0.75rem;background:#f0f2f5;color:#555;padding:2px 8px;border-radius:3px;margin:1px}

/* Severity */
.forge-sev--error{color:#dc2626;font-weight:600}
.forge-sev--warning{color:#d97706;font-weight:600}
.forge-sev--info{color:#2563eb}

/* Check summary */
.forge-checks-summary{font-size:0.9rem;color:#555;margin-bottom:0.5em}

/* Diagram */
.forge-diagram-wrap{margin:1em 0;overflow-x:auto}
.forge-diagram-wrap svg{max-width:100%;height:auto}

/* Diff */
.forge-diff-banner{background:#f0f7ff;border:1px solid #b8d4f0;border-radius:8px;padding:1.25rem 1.5rem;margin:1.5em 0}
.forge-diff-banner h2{margin:0 0 0.5em;border:none;padding:0;font-size:1.2rem}
.forge-diff-desc{margin:0.3em 0 0.8em;color:#333;font-size:1rem}
.forge-diff-stats{display:flex;gap:1.5rem;font-weight:600;font-size:0.9rem}
.forge-diff--added{color:#16a34a}
.forge-diff--modified{color:#d97706}
.forge-diff--removed{color:#dc2626}
.forge-diff-rationale{margin-top:1em;padding-top:0.8em;border-top:1px solid #c8ddf0;font-size:0.95rem}
.forge-diff-legend{display:flex;gap:1.5rem;font-size:0.85rem;font-weight:600;margin:0.5em 0 1em;padding:0.5em 0.75em;background:#f6f8fa;border-radius:4px;width:fit-content}
.forge-diff-highlight--added{outline:3px solid #16a34a;outline-offset:2px;border-radius:4px}
.forge-diff-highlight--modified{outline:3px solid #d97706;outline-offset:2px;border-radius:4px}

/* Documentation */
.forge-doc{line-height:1.7}
.forge-doc h2{margin:1.8em 0 0.5em;font-size:1.25rem;font-weight:600;border-bottom:1px solid #e2e6ea;padding-bottom:0.3em}
.forge-doc h3{margin:1.4em 0 0.4em;font-size:1.1rem;font-weight:600}
.forge-doc p{margin:0.6em 0}
.forge-doc ul,.forge-doc ol{margin:0.5em 0;padding-left:1.5em}
.forge-doc li{margin:0.3em 0}
.forge-doc pre{background:#1a1a2e;color:#e0e0e0;padding:1rem;border-radius:6px;overflow-x:auto;font-size:0.85rem;line-height:1.5}
.forge-doc code{background:#f0f2f5;padding:2px 6px;border-radius:3px;font-size:0.88em}
.forge-doc pre code{background:none;padding:0;font-size:1em}
.forge-doc blockquote{border-left:3px solid #1168BD;margin:1em 0;padding:0.5em 1em;background:#f6f8fa;color:#555}
.forge-doc table{border-collapse:collapse;margin:1em 0;width:100%}
.forge-doc th,.forge-doc td{padding:0.5rem 0.75rem;border:1px solid #e2e6ea;text-align:left}
.forge-doc th{background:#f6f8fa;font-weight:600}
.forge-doc img{max-width:100%}

/* Responsive */
@media(max-width:768px){
  .forge-layout{flex-direction:column}
  .forge-nav{width:100%;height:auto;position:static;display:flex;flex-wrap:wrap;gap:0.5rem;padding:0.75rem}
  .forge-nav__section{display:none}
  .forge-main{padding:1rem}
}
"#;

// ─── Diff SVG Highlighting ───────────────────────────────────────

/// Post-process SVG to inject highlight classes on elements that changed.
/// Looks for `data-id="xxx"` attributes and adds a CSS class to the parent <g>.
fn inject_diff_highlights(svg: &str, diff: &DiffResult) -> String {
    let mut result = svg.to_string();
    for id in &diff.added_ids {
        let needle = format!(r#"data-id="{}""#, id);
        if let Some(pos) = result.find(&needle) {
            // Find the preceding class=" in this <g> tag
            if let Some(class_start) = result[..pos].rfind("class=\"") {
                let insert_at = class_start + 7; // after class="
                result.insert_str(insert_at, "forge-diff-highlight--added ");
            }
        }
    }
    for id in &diff.modified_ids {
        let needle = format!(r#"data-id="{}""#, id);
        if let Some(pos) = result.find(&needle) {
            if let Some(class_start) = result[..pos].rfind("class=\"") {
                let insert_at = class_start + 7;
                result.insert_str(insert_at, "forge-diff-highlight--modified ");
            }
        }
    }
    // Inject diff CSS into the SVG <style> block
    if !diff.added_ids.is_empty() || !diff.modified_ids.is_empty() {
        let diff_css = r#"
    .forge-diff-highlight--added > rect, .forge-diff-highlight--added > circle, .forge-diff-highlight--added > path { stroke: #16a34a !important; stroke-width: 3 !important; }
    .forge-diff-highlight--modified > rect, .forge-diff-highlight--modified > circle, .forge-diff-highlight--modified > path { stroke: #d97706 !important; stroke-width: 3 !important; }
"#;
        if let Some(style_end) = result.find("</style>") {
            result.insert_str(style_end, diff_css);
        }
    }
    result
}

// ─── Utilities ───────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn kind_display(kind: ElementKind) -> &'static str {
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
        _ => "Element",
    }
}

fn kind_css(kind: ElementKind) -> &'static str {
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

fn kind_order(kind: ElementKind) -> u8 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn payments_model() -> Model {
        let text = include_str!("../examples/payments.forge");
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
        assert_eq!(report.diagrams, 4);

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
        let text = include_str!("../examples/payments-baseline.forge");
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
        let svg = r#"<svg><defs><style></style></defs><g class="forge-element" data-id="new.svc"></g></svg>"#;
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
}

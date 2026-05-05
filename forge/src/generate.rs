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

use crate::animate;
use crate::check;
use crate::diff::DiffResult;
use crate::model::*;
use crate::render;

mod catalog;
mod css;
mod diff_svg;
mod json;
mod pages;
mod template;
mod util;

#[cfg(test)]
mod tests;

pub use catalog::{generate_catalog, CatalogGenerateConfig};

use css::CSS;
use diff_svg::inject_diff_highlights;
use json::render_json;
use pages::{
    build_nav, render_doc_page, render_element_page, render_index, render_markdown,
    render_view_page, slugify_doc,
};
use util::kind_order;

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
            style: "outline".into(),
            source_dir: PathBuf::from("."),
        }
    }
}

pub struct GenerateReport {
    pub pages: usize,
    pub diagrams: usize,
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
        let mut svg = render::render_view(model, view, &config.style);
        if !view.animation.is_empty() || view.kind == ViewKind::Dynamic {
            svg = animate::animate_svg(&svg, view, model);
        }
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

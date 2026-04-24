use super::template::page_template;
use super::util::{esc, kind_css, kind_display};
use super::*;
use crate::check;
use crate::diff::{ChangeKind, DiffResult};
pub(super) fn build_nav(model: &Model, base: &str) -> String {
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
pub(super) fn render_index(
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

    // External dependencies
    if !model.dependencies.is_empty() {
        main.push_str("<h2>External Dependencies</h2>");
        main.push_str(r#"<table class="forge-table"><thead><tr><th>Name</th><th>Type</th><th>Criticality</th><th>Description</th></tr></thead><tbody>"#);
        for dep in &model.dependencies {
            let crit_cls = match dep.criticality.as_str() {
                "critical" => "forge-sev--error",
                "high" => "forge-sev--warning",
                _ => "",
            };
            main.push_str(&format!(
                r#"<tr><td>{}</td><td>{}</td><td class="{}">{}</td><td>{}</td></tr>"#,
                esc(&dep.name),
                esc(&dep.kind),
                crit_cls,
                esc(&dep.criticality),
                esc(dep.description.as_deref().unwrap_or(""))
            ));
        }
        main.push_str("</tbody></table>");
    }

    page_template(title, &main, nav, base)
}

// ─── View Page ───────────────────────────────────────────────────
pub(super) fn render_view_page(
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

    // Inject playback script for animated views
    if !view.animation.is_empty() {
        main.push_str(&format!(
            "<p class=\"forge-anim-hint\">Click the diagram or use arrow keys to step through frames.</p>\n<script>{}</script>",
            animate::playback_script()
        ));
    }

    page_template(&format!("{} — {}", view_title, title), &main, nav, base)
}

// ─── Element Page ────────────────────────────────────────────────

pub(super) fn render_element_page(
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

    // ── Phase 6: SLOs ──
    let slos: Vec<_> = model.slos.iter().filter(|s| s.container == el.id).collect();
    if !slos.is_empty() {
        main.push_str("<h2>Service Level Objectives</h2>");
        main.push_str(r#"<table class="forge-table"><tbody>"#);
        for slo in &slos {
            if let Some(ref l) = slo.latency {
                main.push_str(&format!(
                    "<tr><th>Latency (p99)</th><td>{}</td></tr>",
                    esc(l)
                ));
            }
            if let Some(ref a) = slo.availability {
                main.push_str(&format!(
                    "<tr><th>Availability</th><td>{}</td></tr>",
                    esc(a)
                ));
            }
            if let Some(ref e) = slo.error_budget {
                main.push_str(&format!(
                    "<tr><th>Error Budget</th><td>{}</td></tr>",
                    esc(e)
                ));
            }
        }
        main.push_str("</tbody></table>");
    }

    // ── Phase 6: API Endpoints ──
    let apis: Vec<_> = model
        .api_catalogs
        .iter()
        .filter(|a| a.container == el.id)
        .collect();
    if !apis.is_empty() {
        main.push_str("<h2>API Endpoints</h2>");
        main.push_str(r#"<table class="forge-table"><thead><tr><th>Endpoint</th><th>Description</th></tr></thead><tbody>"#);
        for catalog in &apis {
            for ep in &catalog.endpoints {
                main.push_str(&format!(
                    "<tr><td><code>{} {}</code></td><td>{}</td></tr>",
                    esc(&ep.method),
                    esc(&ep.path),
                    esc(ep.description.as_deref().unwrap_or(""))
                ));
            }
        }
        main.push_str("</tbody></table>");
    }

    // ── Phase 6: Runbook / operational properties ──
    if let Some(runbook) = el.properties.get("runbook") {
        main.push_str(&format!(
            "<h2>Runbook</h2><p><a href=\"{}\">{}</a></p>",
            esc(runbook),
            esc(runbook)
        ));
    }

    page_template(&format!("{} — {}", el.name, title), &main, nav, base)
}

// ─── Doc Page ────────────────────────────────────────────────────
pub(super) fn render_doc_page(
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

pub(super) fn render_markdown(md: &str) -> String {
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

pub(super) fn slugify_doc(title: &str) -> String {
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

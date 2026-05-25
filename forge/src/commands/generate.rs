//! `forge generate` and `forge generate-catalog` — produce static HTML
//! documentation sites from a model (or catalog of models).

use std::fs;
use std::path::Path;

use crate::{catalog_parser, diff, generate};

use super::util::{die, load_model};

pub fn cmd_generate(
    source: &Path,
    out: &Path,
    title: Option<String>,
    base_url: &str,
    style: &str,
    baseline: Option<&Path>,
) {
    let model = load_model(source);

    let diff_result = baseline.map(|bp| {
        let bm = load_model(bp);
        let dr = diff::diff(&bm, &model);
        eprintln!(
            "Diff: {} added, {} modified, {} removed",
            dr.added_count(),
            dr.modified_count(),
            dr.removed_count()
        );
        dr
    });

    eprintln!("Generating site from \"{}\"...", model.name);
    let source_dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    let config = generate::GenerateConfig {
        out_dir: out.to_path_buf(),
        title,
        base_url: base_url.into(),
        style: style.into(),
        source_dir,
    };

    match generate::generate(&model, &config, diff_result.as_ref()) {
        Ok(report) => {
            eprintln!(
                "  {} pages, {} diagrams → {}",
                report.pages,
                report.diagrams,
                out.display()
            );
            eprintln!("Done.");
        }
        Err(e) => die(&format!("generate: {}", e)),
    }
}

pub fn cmd_generate_catalog(
    source: &Path,
    out: &Path,
    title: Option<String>,
    base_url: &str,
    style: &str,
    incremental: bool,
) {
    // Load and parse the catalog file
    let catalog_text = fs::read_to_string(source)
        .unwrap_or_else(|e| die(&format!("reading {}: {}", source.display(), e)));

    let catalog = catalog_parser::parse_catalog(&catalog_text)
        .unwrap_or_else(|e| die(&format!("parsing catalog: {}", e)));

    eprintln!("Generating catalog site from \"{}\"...", catalog.name);
    eprintln!("  {} projects", catalog.projects.len());
    if incremental {
        eprintln!("  Incremental mode: skipping unchanged projects");
    }

    let config = generate::CatalogGenerateConfig {
        out_dir: out.to_path_buf(),
        title,
        base_url: base_url.into(),
        style: style.into(),
        incremental,
    };

    match generate::generate_catalog(&catalog, &config, None) {
        Ok(report) => {
            eprintln!(
                "  Processed: {} projects ({} skipped)",
                report.projects_processed, report.projects_skipped
            );
            eprintln!(
                "  Generated: {} pages, {} diagrams → {}",
                report.total_pages,
                report.total_diagrams,
                out.display()
            );
            eprintln!("Done.");
        }
        Err(e) => die(&format!("generate-catalog: {}", e)),
    }
}

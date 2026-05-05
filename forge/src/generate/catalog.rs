//! Multi-project catalog site generator for enterprise-scale deployments.
//!
//! Supports generating a unified documentation site from multiple separate .forge
//! models, with incremental updates and cross-project navigation.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{Catalog, CatalogProject, Model};
use crate::parser;
use crate::preprocess;

use super::{generate, GenerateConfig};

/// Configuration for catalog site generation.
pub struct CatalogGenerateConfig {
    pub out_dir: PathBuf,
    pub title: Option<String>,
    pub base_url: String,
    pub style: String,
    pub incremental: bool,
}

impl Default for CatalogGenerateConfig {
    fn default() -> Self {
        Self {
            out_dir: PathBuf::from("_site"),
            title: None,
            base_url: "/".into(),
            style: "outline".into(),
            incremental: true,
        }
    }
}

/// Report for catalog generation with per-project statistics.
pub struct CatalogGenerateReport {
    pub projects_processed: usize,
    pub projects_skipped: usize,
    pub total_pages: usize,
    pub total_diagrams: usize,
}

/// Generate a multi-project catalog site.
///
/// This function orchestrates the generation of a documentation site that
/// aggregates multiple separate .forge models. Each project gets its own
/// subdirectory, and a unified index page provides navigation across all projects.
///
/// When `incremental` is true, only projects with modified source files are
/// regenerated, significantly reducing build times for large catalogs.
pub fn generate_catalog(
    catalog: &Catalog,
    config: &CatalogGenerateConfig,
    baselines: Option<&HashMap<String, Model>>,
) -> Result<CatalogGenerateReport, String> {
    let out = &config.out_dir;

    // Create root directory structure
    fs::create_dir_all(out).map_err(|e| format!("mkdir: {}", e))?;
    fs::create_dir_all(out.join("projects")).map_err(|e| format!("mkdir projects: {}", e))?;

    let mut total_pages = 0;
    let mut total_diagrams = 0;
    let mut projects_processed = 0;
    let mut projects_skipped = 0;

    // Generate each project's site in its own subdirectory
    for project in &catalog.projects {
        // Check if we should skip this project (incremental mode)
        if config.incremental && should_skip_project(project, out)? {
            projects_skipped += 1;
            continue;
        }

        // Load and parse the project model
        let model = load_project_model(&project.source)?;

        // Set up project-specific output directory
        let project_out = out.join("projects").join(&project.key);
        let project_config = GenerateConfig {
            out_dir: project_out.clone(),
            title: Some(project.name.clone()),
            base_url: format!(
                "{}/projects/{}/",
                config.base_url.trim_end_matches('/'),
                project.key
            ),
            style: config.style.clone(),
            source_dir: PathBuf::from(&project.source)
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf(),
        };

        // Get baseline for diff highlighting if available
        let diff = baselines.as_ref().and_then(|map| {
            map.get(&project.key)
                .map(|baseline| crate::diff::diff(baseline, &model))
        });

        // Generate the project site
        let report = generate(&model, &project_config, diff.as_ref())?;
        total_pages += report.pages;
        total_diagrams += report.diagrams;
        projects_processed += 1;

        // Store metadata for incremental builds
        store_project_metadata(project, &project_out)?;
    }

    // Generate catalog index page
    generate_catalog_index(catalog, config)?;
    total_pages += 1;

    // Copy shared assets
    copy_shared_assets(out)?;

    Ok(CatalogGenerateReport {
        projects_processed,
        projects_skipped,
        total_pages,
        total_diagrams,
    })
}

/// Load and parse a project model from its source file.
fn load_project_model(source_path: &str) -> Result<Model, String> {
    let path = Path::new(source_path);
    let source_dir = path.parent().unwrap_or(Path::new("."));

    // Read the source file
    let source_text =
        fs::read_to_string(path).map_err(|e| format!("read {}: {}", source_path, e))?;

    // Preprocess to handle !include directives
    let preprocessed = preprocess::preprocess(&source_text, source_dir)
        .map_err(|e| format!("preprocess {}: {}", source_path, e))?;

    // Parse the model
    parser::parse(&preprocessed).map_err(|e| format!("parse {}: {}", source_path, e))
}

/// Check if a project should be skipped in incremental mode.
fn should_skip_project(project: &CatalogProject, out_dir: &Path) -> Result<bool, String> {
    let project_out = out_dir.join("projects").join(&project.key);
    let metadata_file = project_out.join(".forge-meta");

    // If output doesn't exist, must regenerate
    if !project_out.exists() || !metadata_file.exists() {
        return Ok(false);
    }

    // Check source file modification time
    let source_path = Path::new(&project.source);
    let source_meta =
        fs::metadata(source_path).map_err(|e| format!("stat {}: {}", project.source, e))?;
    let source_mtime = source_meta
        .modified()
        .map_err(|e| format!("mtime {}: {}", project.source, e))?;

    // Read stored metadata
    let stored = fs::read_to_string(&metadata_file).map_err(|e| format!("read metadata: {}", e))?;
    let stored_time: u64 = stored
        .parse()
        .map_err(|e| format!("parse metadata: {}", e))?;

    // Compare times
    let current_time = source_mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("time conversion: {}", e))?
        .as_secs();

    Ok(current_time <= stored_time)
}

/// Store project metadata for incremental builds.
fn store_project_metadata(project: &CatalogProject, project_out: &Path) -> Result<(), String> {
    let metadata_file = project_out.join(".forge-meta");
    let source_path = Path::new(&project.source);

    let source_meta =
        fs::metadata(source_path).map_err(|e| format!("stat {}: {}", project.source, e))?;
    let source_mtime = source_meta
        .modified()
        .map_err(|e| format!("mtime {}: {}", project.source, e))?;
    let timestamp = source_mtime
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("time conversion: {}", e))?
        .as_secs();

    fs::write(&metadata_file, timestamp.to_string())
        .map_err(|e| format!("write metadata: {}", e))?;

    Ok(())
}

/// Generate the catalog index page.
fn generate_catalog_index(catalog: &Catalog, config: &CatalogGenerateConfig) -> Result<(), String> {
    let out = &config.out_dir;
    let title = config.title.as_deref().unwrap_or_else(|| {
        if catalog.name.is_empty() {
            "Enterprise Architecture Catalog"
        } else {
            &catalog.name
        }
    });

    let html = render_catalog_index(title, catalog, &config.base_url);
    fs::write(out.join("index.html"), &html).map_err(|e| format!("write index: {}", e))?;

    Ok(())
}

/// Render the catalog index HTML.
fn render_catalog_index(title: &str, catalog: &Catalog, base_url: &str) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"en\">\n");
    html.push_str("<head>\n");
    html.push_str("  <meta charset=\"UTF-8\">\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    html.push_str(&format!("  <title>{}</title>\n", html_escape(title)));
    html.push_str(&format!(
        "  <link rel=\"stylesheet\" href=\"{}assets/forge.css\">\n",
        base_url
    ));
    html.push_str("</head>\n");
    html.push_str("<body>\n");
    html.push_str("  <header class=\"forge-header\">\n");
    html.push_str(&format!("    <h1>{}</h1>\n", html_escape(title)));
    if !catalog.description.is_empty() {
        html.push_str(&format!(
            "    <p class=\"forge-description\">{}</p>\n",
            html_escape(&catalog.description)
        ));
    }
    html.push_str("  </header>\n");
    html.push_str("  <main class=\"forge-catalog-index\">\n");
    html.push_str("    <h2>Projects</h2>\n");
    html.push_str("    <div class=\"forge-project-grid\">\n");

    for project in &catalog.projects {
        html.push_str("      <div class=\"forge-project-card\">\n");
        html.push_str(&format!(
            "        <h3><a href=\"{}projects/{}/\">{}</a></h3>\n",
            base_url,
            project.key,
            html_escape(&project.name)
        ));

        if let Some(desc) = &project.description {
            html.push_str(&format!("        <p>{}</p>\n", html_escape(desc)));
        }

        if let Some(repo) = &project.repository {
            html.push_str(&format!(
                "        <p class=\"forge-project-repo\"><code>{}</code></p>\n",
                html_escape(repo)
            ));
        }

        if !project.tags.is_empty() {
            html.push_str("        <div class=\"forge-tags\">\n");
            for tag in &project.tags {
                html.push_str(&format!(
                    "          <span class=\"forge-tag\">{}</span>\n",
                    html_escape(tag)
                ));
            }
            html.push_str("        </div>\n");
        }

        html.push_str("      </div>\n");
    }

    html.push_str("    </div>\n");
    html.push_str("  </main>\n");
    html.push_str("</body>\n");
    html.push_str("</html>\n");

    html
}

/// Copy shared assets to the catalog output directory.
fn copy_shared_assets(out_dir: &Path) -> Result<(), String> {
    let assets_dir = out_dir.join("assets");
    fs::create_dir_all(&assets_dir).map_err(|e| format!("mkdir assets: {}", e))?;

    // Write the CSS file
    fs::write(assets_dir.join("forge.css"), super::css::CSS)
        .map_err(|e| format!("write css: {}", e))?;

    Ok(())
}

/// HTML escape helper.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("Hello <world>"), "Hello &lt;world&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }

    #[test]
    fn test_catalog_config_default() {
        let config = CatalogGenerateConfig::default();
        assert_eq!(config.base_url, "/");
        assert_eq!(config.style, "outline");
        assert!(config.incremental);
    }
}

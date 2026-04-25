//! Forge analyze — scan codebases and produce .forge models.
//!
//! Scanners detect project structure, CI/CD pipelines, Docker containers,
//! and dependencies from real files, producing a Model that can be emitted
//! as .forge DSL.

pub mod ci;
pub mod code;
pub mod container_index;
pub mod correlate;
pub mod docker;
pub mod emit;
pub mod git;
pub mod infra;
pub mod k8s;
pub mod merge;
pub mod plugin;
pub mod provenance;
pub mod semantic;
pub mod synth;

#[cfg(test)]
mod fixture_tests;

use std::path::{Path, PathBuf};

use crate::model::Model;

use container_index::ContainerIndex;

/// Configuration for an analyze run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AnalyzeConfig {
    pub paths: Vec<PathBuf>,
    pub scanners: Vec<String>,
    pub out: PathBuf,
    pub dry_run: bool,
    pub exclude: Vec<String>,
    pub plugins: Vec<plugin::PluginCommand>,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            scanners: vec![
                "code".into(),
                "semantic".into(),
                "ci".into(),
                "docker".into(),
                "git".into(),
                "k8s".into(),
                "infra".into(),
            ],
            out: PathBuf::from("forge.forge"),
            dry_run: false,
            exclude: vec![
                "node_modules".into(),
                "target".into(),
                ".git".into(),
                "vendor".into(),
                "dist".into(),
                "__pycache__".into(),
            ],
            plugins: Vec::new(),
        }
    }
}

impl AnalyzeConfig {
    pub fn should_exclude(&self, path: &Path) -> bool {
        for component in path.components() {
            let s = component.as_os_str().to_string_lossy();
            if self.exclude.iter().any(|e| s.as_ref() == e.as_str()) {
                return true;
            }
        }
        false
    }
}

/// Convert a name to a URL-safe slug.
pub fn slugify(name: &str) -> String {
    let mut s: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

/// Run all configured scanners and merge results into a single Model.
pub fn analyze(config: &AnalyzeConfig) -> Model {
    let mut model = Model::default();
    let mut index = ContainerIndex::new();

    for scan_path in &config.paths {
        let dir_name = scan_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".into());
        if model.name.is_empty() {
            model.name = dir_name;
        }

        // `code` must run before `semantic` so containers exist when source
        // attribution runs. We preserve user ordering otherwise.
        let ordered = order_scanners(&config.scanners);

        for scanner_name in &ordered {
            match scanner_name.as_str() {
                "code" => code::scan(&mut model, &mut index, scan_path, config),
                "semantic" => semantic::scan(&mut model, &index, scan_path, config),
                "ci" => ci::scan(&mut model, scan_path, config),
                "docker" => docker::scan(&mut model, scan_path, config),
                "git" => git::scan(&mut model, scan_path, config),
                "k8s" => k8s::scan(&mut model, scan_path, config),
                "infra" => infra::scan(&mut model, scan_path, config),
                _ => {
                    eprintln!("  Warning: unknown scanner '{}'", scanner_name);
                }
            }
        }
    }

    // Cross-scanner post-pass: drain scanner-local facts (env var reads and
    // provides, etc.) into proper relationships now that every scanner has
    // had a chance to populate the model.
    correlate::run(&mut model);

    // External plugins run after built-in scanners so they can reference
    // discovered containers, but before synth so their additions participate
    // in tech-stack aggregates and default views.
    if !config.plugins.is_empty() {
        for scan_path in &config.paths {
            plugin::run(&config.plugins, scan_path, config, &mut model);
        }
    }

    // Synthesis post-pass: fill in high-level structure (System wrapper,
    // default Person actors, tech-stack aggregates, default Views) so that
    // freshly-analyzed models render useful views without hand-editing.
    synth::run(&mut model);

    model
}

/// Ensure `code` runs before `semantic` so containers exist before source
/// attribution, preserving the relative order of every other scanner.
fn order_scanners(requested: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(requested.len());
    let has_code = requested.iter().any(|s| s == "code");
    let has_semantic = requested.iter().any(|s| s == "semantic");
    if has_code && has_semantic {
        for s in requested {
            if s == "semantic" {
                continue;
            }
            out.push(s.clone());
            if s == "code" {
                out.push("semantic".into());
            }
        }
    } else {
        out.extend(requested.iter().cloned());
    }
    out
}

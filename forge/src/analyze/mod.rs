//! Forge analyze — scan codebases and produce .forge models.
//!
//! Scanners detect project structure, CI/CD pipelines, Docker containers,
//! and dependencies from real files, producing a Model that can be emitted
//! as .forge DSL.

pub mod ci;
pub mod code;
pub mod docker;
pub mod emit;
pub mod git;

use std::path::{Path, PathBuf};

use crate::model::Model;

/// Configuration for an analyze run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AnalyzeConfig {
    pub paths: Vec<PathBuf>,
    pub scanners: Vec<String>,
    pub out: PathBuf,
    pub dry_run: bool,
    pub exclude: Vec<String>,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            scanners: vec!["code".into(), "ci".into(), "docker".into(), "git".into()],
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

    for scan_path in &config.paths {
        let dir_name = scan_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".into());
        if model.name.is_empty() {
            model.name = dir_name;
        }

        for scanner_name in &config.scanners {
            match scanner_name.as_str() {
                "code" => code::scan(&mut model, scan_path, config),
                "ci" => ci::scan(&mut model, scan_path, config),
                "docker" => docker::scan(&mut model, scan_path, config),
                "git" => git::scan(&mut model, scan_path, config),
                _ => {
                    eprintln!("  Warning: unknown scanner '{}'", scanner_name);
                }
            }
        }
    }

    model
}

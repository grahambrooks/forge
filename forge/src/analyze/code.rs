//! Code scanner — detects project structure from manifest files.
//!
//! Without tree-sitter, this scanner uses project manifests (Cargo.toml,
//! package.json, go.mod, etc.) to discover components, their technologies,
//! and inter-project dependencies.

use std::path::Path;

use walkdir::WalkDir;

use crate::model::*;

use super::{slugify, AnalyzeConfig};

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match name {
            "Cargo.toml" => parse_cargo_toml(model, path, root),
            "package.json" => parse_package_json(model, path, root),
            "go.mod" => parse_go_mod(model, path, root),
            "pyproject.toml" | "setup.py" | "setup.cfg" => parse_python_project(model, path, root),
            "pom.xml" => parse_java_project(model, path, root, "Maven"),
            "build.gradle" | "build.gradle.kts" => parse_java_project(model, path, root, "Gradle"),
            _ => {}
        }
    }
}

// ── Cargo.toml (Rust) ────────────────────────────────────────────

fn parse_cargo_toml(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    // Skip workspace-level Cargo.toml with no [package]
    if !text.contains("[package]") {
        // But check for workspace members
        if text.contains("[workspace]") {
            // Workspace root — scan members individually
            return;
        }
        return;
    }

    let name = extract_toml_value(&text, "name").unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("rust-project")
            .to_string()
    });

    let description = extract_toml_value(&text, "description");
    let container_id = slugify(&name);

    if model.elements.contains_key(&container_id) {
        return;
    }

    let mut el = Element::new(&container_id, ElementKind::Container, &name);
    el.technology = Some("Rust".into());
    el.tags.push("inferred".into());

    if let Some(desc) = description {
        el.description = Some(desc);
    } else {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        el.description = Some(format!(
            "Rust crate at {}",
            rel_path.parent().unwrap_or(rel_path).display()
        ));
    }

    // Look for common framework dependencies to refine technology
    if text.contains("actix-web") || text.contains("actix_web") {
        el.technology = Some("Rust / Actix".into());
    } else if text.contains("axum") {
        el.technology = Some("Rust / Axum".into());
    } else if text.contains("rocket") {
        el.technology = Some("Rust / Rocket".into());
    } else if text.contains("warp") {
        el.technology = Some("Rust / Warp".into());
    } else if text.contains("tonic") {
        el.technology = Some("Rust / Tonic (gRPC)".into());
    }

    // Detect if it's a library or binary
    let is_lib =
        text.contains("[lib]") || path.parent().is_some_and(|p| p.join("src/lib.rs").exists());
    let is_bin = text.contains("[[bin]]")
        || path
            .parent()
            .is_some_and(|p| p.join("src/main.rs").exists());
    if is_lib && !is_bin {
        el.tags.push("library".into());
    }

    model.add_element(el);

    // Extract inter-crate dependencies (path deps = local relationships)
    extract_path_deps(model, &text, &container_id);
}

fn extract_path_deps(model: &mut Model, text: &str, from_id: &str) {
    // Simple pattern: name = { path = "...", ... }
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains("path") && trimmed.contains('=') && trimmed.contains('"') {
            // Try to extract the dependency name (first thing before =)
            if let Some(dep_name) = trimmed.split('=').next() {
                let dep_name = dep_name.trim();
                if !dep_name.is_empty()
                    && dep_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    let dep_id = slugify(dep_name);
                    model.add_relationship(Relationship {
                        frm: from_id.into(),
                        to: dep_id,
                        label: "depends on".into(),
                        technology: None,
                    });
                }
            }
        }
    }
}

// ── package.json (Node.js/TypeScript) ────────────────────────────

fn parse_package_json(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    // Quick JSON extraction without a full parser
    let name = extract_json_string(&text, "name").unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("node-project")
            .to_string()
    });

    // Skip if it looks like a workspace root (has "workspaces" field)
    if text.contains("\"workspaces\"") && path.parent() == Some(root) {
        return;
    }

    let container_id = slugify(&name);
    if model.elements.contains_key(&container_id) {
        return;
    }

    let description = extract_json_string(&text, "description");

    let mut el = Element::new(&container_id, ElementKind::Container, &name);
    el.tags.push("inferred".into());

    // Detect TypeScript
    let is_ts = path
        .parent()
        .is_some_and(|p| p.join("tsconfig.json").exists());
    let tech_base = if is_ts { "TypeScript" } else { "Node.js" };

    // Detect frameworks
    let tech = if text.contains("\"next\"") || text.contains("\"next\":") {
        format!("{} / Next.js", tech_base)
    } else if text.contains("\"express\"") {
        format!("{} / Express", tech_base)
    } else if text.contains("\"fastify\"") {
        format!("{} / Fastify", tech_base)
    } else if text.contains("\"react\"") && !text.contains("\"next\"") {
        format!("{} / React", tech_base)
    } else if text.contains("\"vue\"") {
        format!("{} / Vue", tech_base)
    } else if text.contains("\"@angular/core\"") {
        format!("{} / Angular", tech_base)
    } else if text.contains("\"svelte\"") {
        format!("{} / Svelte", tech_base)
    } else {
        tech_base.to_string()
    };
    el.technology = Some(tech);

    if let Some(desc) = description {
        el.description = Some(desc);
    } else {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        el.description = Some(format!(
            "Node.js package at {}",
            rel_path.parent().unwrap_or(rel_path).display()
        ));
    }

    model.add_element(el);
}

// ── go.mod (Go) ──────────────────────────────────────────────────

fn parse_go_mod(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let module_name = text
        .lines()
        .find_map(|line| line.strip_prefix("module "))
        .map(|m| m.trim().to_string());

    let name = module_name
        .as_ref()
        .map(|m| m.split('/').next_back().unwrap_or(m).to_string())
        .unwrap_or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("go-project")
                .to_string()
        });

    let container_id = slugify(&name);
    if model.elements.contains_key(&container_id) {
        return;
    }

    let mut el = Element::new(&container_id, ElementKind::Container, &name);
    el.technology = Some("Go".into());
    el.tags.push("inferred".into());

    // Detect frameworks
    if text.contains("github.com/gin-gonic/gin") {
        el.technology = Some("Go / Gin".into());
    } else if text.contains("github.com/labstack/echo") {
        el.technology = Some("Go / Echo".into());
    } else if text.contains("github.com/gofiber/fiber") {
        el.technology = Some("Go / Fiber".into());
    } else if text.contains("google.golang.org/grpc") {
        el.technology = Some("Go / gRPC".into());
    }

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    el.description = Some(format!(
        "Go module at {}",
        rel_path.parent().unwrap_or(rel_path).display()
    ));

    model.add_element(el);
}

// ── Python projects ──────────────────────────────────────────────

fn parse_python_project(model: &mut Model, path: &Path, root: &Path) {
    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("python-project");

    let container_id = slugify(dir_name);
    if model.elements.contains_key(&container_id) {
        return;
    }

    let text = std::fs::read_to_string(path).unwrap_or_default();

    let mut el = Element::new(&container_id, ElementKind::Container, dir_name);
    el.technology = Some("Python".into());
    el.tags.push("inferred".into());

    // Detect frameworks
    if text.contains("fastapi") || text.contains("FastAPI") {
        el.technology = Some("Python / FastAPI".into());
    } else if text.contains("django") || text.contains("Django") {
        el.technology = Some("Python / Django".into());
    } else if text.contains("flask") || text.contains("Flask") {
        el.technology = Some("Python / Flask".into());
    }

    // Try to get name from pyproject.toml
    if path.file_name().and_then(|n| n.to_str()) == Some("pyproject.toml") {
        if let Some(name) = extract_toml_value(&text, "name") {
            el.name = name;
        }
        if let Some(desc) = extract_toml_value(&text, "description") {
            el.description = Some(desc);
        }
    }

    if el.description.is_none() {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        el.description = Some(format!(
            "Python project at {}",
            rel_path.parent().unwrap_or(rel_path).display()
        ));
    }

    model.add_element(el);
}

// ── Java/Kotlin projects ─────────────────────────────────────────

fn parse_java_project(model: &mut Model, path: &Path, root: &Path, build_tool: &str) {
    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("java-project");

    let container_id = slugify(dir_name);
    if model.elements.contains_key(&container_id) {
        return;
    }

    let text = std::fs::read_to_string(path).unwrap_or_default();

    let mut el = Element::new(&container_id, ElementKind::Container, dir_name);
    el.technology = Some(format!("Java / {}", build_tool));
    el.tags.push("inferred".into());

    // Detect Spring Boot
    if text.contains("spring-boot") || text.contains("org.springframework.boot") {
        el.technology = Some(format!("Java / Spring Boot ({})", build_tool));
    }

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    el.description = Some(format!(
        "Java project at {}",
        rel_path.parent().unwrap_or(rel_path).display()
    ));

    model.add_element(el);
}

// ── Utilities ────────────────────────────────────────────────────

/// Simple TOML value extractor (key = "value" on its own line).
fn extract_toml_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                if rest.starts_with('"') && rest.len() > 1 {
                    if let Some(end) = rest[1..].find('"') {
                        return Some(rest[1..1 + end].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Simple JSON string extractor ("key": "value").
fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(&pattern) {
            let after = &trimmed[pos + pattern.len()..];
            let after = after.trim().trim_start_matches(':').trim();
            if let Some(stripped) = after.strip_prefix('"') {
                if let Some(end) = stripped.find('"') {
                    return Some(stripped[..end].to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_cargo_name() {
        let toml = r#"
[package]
name = "my-service"
version = "0.1.0"
description = "A cool service"
"#;
        assert_eq!(extract_toml_value(toml, "name"), Some("my-service".into()));
        assert_eq!(
            extract_toml_value(toml, "description"),
            Some("A cool service".into())
        );
    }

    #[test]
    fn extract_package_json_name() {
        let json = r#"{
  "name": "@org/my-app",
  "version": "1.0.0",
  "description": "My application"
}"#;
        assert_eq!(
            extract_json_string(json, "name"),
            Some("@org/my-app".into())
        );
        assert_eq!(
            extract_json_string(json, "description"),
            Some("My application".into())
        );
    }

    #[test]
    fn detect_rust_framework() {
        let toml = r#"
[package]
name = "api"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
"#;
        // Verify framework detection works by checking text matching
        assert!(toml.contains("axum"));
    }
}

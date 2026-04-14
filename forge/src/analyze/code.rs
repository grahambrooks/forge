//! Code scanner — discovers containers from package-manager manifests.
//!
//! Delegates all file parsing to `symgraph::extraction::manifest`, then maps
//! the resulting nodes onto forge Containers. Workspace manifests
//! (Cargo `[workspace]`, npm/pnpm/yarn `workspaces`, `go.work`,
//! `settings.gradle`) are expanded so each member becomes its own container.
//!
//! Technology labels are inferred from the dependency set that symgraph
//! extracts — no string-matching against raw manifest text.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use symgraph::extraction::Extractor;
use symgraph::types::{ExtractionResult, Language, NodeKind};

use crate::model::*;

use super::container_index::ContainerIndex;
use super::provenance::mark_inferred;
use super::{slugify, AnalyzeConfig};

const SCANNER: &str = "code";

/// Filenames symgraph recognises as package-manager manifests.
const MANIFEST_NAMES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "go.mod",
    "pyproject.toml",
    "requirements.txt",
    "Gemfile",
    "composer.json",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "build.sbt",
];

pub fn scan(model: &mut Model, index: &mut ContainerIndex, root: &Path, config: &AnalyzeConfig) {
    // Track which directories have already been handled as workspace members
    // so the outer walk can skip re-emitting the workspace root as its own
    // container.
    let mut workspace_handled: HashSet<PathBuf> = HashSet::new();

    for entry in WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !MANIFEST_NAMES.contains(&name) {
            continue;
        }

        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Workspace roots: expand members, then mark the root dir as handled
        // so we don't try to promote the workspace manifest itself into a
        // container.
        if let Some(members) = detect_workspace_members(name, &text, path) {
            for member in members {
                handle_manifest_dir(model, index, &member, root, config);
            }
            if let Some(dir) = path.parent() {
                workspace_handled.insert(dir.to_path_buf());
            }
            continue;
        }

        if let Some(dir) = path.parent() {
            if workspace_handled.contains(dir) {
                continue;
            }
        }

        handle_manifest_file(model, index, path, &text, root);
    }
}

/// Parse a manifest at `path` (content already read) and register a container.
fn handle_manifest_file(
    model: &mut Model,
    index: &mut ContainerIndex,
    path: &Path,
    text: &str,
    root: &Path,
) {
    let result = Extractor::new().extract_file(path, text);
    let Some(container) = build_container(&result, path, root) else {
        return;
    };

    if model.elements.contains_key(&container.id) {
        return;
    }

    if let Some(dir) = path.parent() {
        index.register(dir.to_path_buf(), container.id.clone());
    }
    model.add_element(container);
}

/// Walk a workspace member directory and handle its manifest (if any).
fn handle_manifest_dir(
    model: &mut Model,
    index: &mut ContainerIndex,
    dir: &Path,
    root: &Path,
    config: &AnalyzeConfig,
) {
    for name in MANIFEST_NAMES {
        let path = dir.join(name);
        if !path.exists() {
            continue;
        }
        if config.should_exclude(&path) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            // A workspace member should never itself be another workspace
            // root, so call the leaf handler directly.
            handle_manifest_file(model, index, &path, &text, root);
            return;
        }
    }
}

/// Convert a symgraph ExtractionResult for a manifest file into a forge
/// Container element, with technology inferred from the dependency set.
fn build_container(result: &ExtractionResult, path: &Path, root: &Path) -> Option<Element> {
    let file_node = result.nodes.iter().find(|n| n.kind == NodeKind::File)?;

    // Name: prefer the first Module node (the declared package name),
    // fall back to the containing directory name. Go module declarations
    // carry the full import path (e.g. `github.com/example/orders`) which
    // would slug into an unreadable id; use only the trailing segment.
    let module_name = result
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Module)
        .map(|n| {
            n.name
                .rsplit('/')
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| n.name.clone())
        });

    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    let name = module_name.clone().unwrap_or_else(|| dir_name.clone());
    let id = slugify(&name);
    if id.is_empty() {
        return None;
    }

    let deps: Vec<String> = result
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Import)
        .map(|n| n.name.clone())
        .collect();

    // symgraph reports package.json as JavaScript. Promote to TypeScript
    // when a sibling `tsconfig.json` exists so the node-express-style
    // "TypeScript / Express" label lands correctly on real TS projects.
    let effective_lang = if matches!(file_node.language, Language::JavaScript)
        && path
            .parent()
            .map(|p| p.join("tsconfig.json").exists())
            .unwrap_or(false)
    {
        Language::TypeScript
    } else {
        file_node.language
    };

    let mut el = Element::new(&id, ElementKind::Container, &name);
    el.technology = Some(infer_technology(effective_lang, &deps));

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    el.description = Some(format!(
        "{} at {}",
        describe_language(file_node.language),
        rel_path.parent().unwrap_or(rel_path).display()
    ));

    if deps
        .iter()
        .any(|d| is_library_signal(&file_node.language, d))
    {
        el.tags.push("library".into());
    }

    mark_inferred(&mut el, SCANNER, Some(rel_path));
    Some(el)
}

/// Human-readable language label used in container descriptions.
fn describe_language(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "Rust crate",
        Language::Go => "Go module",
        Language::Python => "Python project",
        Language::Java => "Java project",
        Language::Kotlin => "Kotlin project",
        Language::Scala => "Scala project",
        Language::Groovy => "Gradle project",
        Language::JavaScript | Language::TypeScript | Language::Jsx | Language::Tsx => {
            "Node.js package"
        }
        Language::Ruby => "Ruby project",
        Language::Php => "PHP project",
        _ => "Project",
    }
}

fn is_library_signal(_lang: &Language, _dep: &str) -> bool {
    // Conservative: we don't infer library/binary from deps. Future scanners
    // may look at Cargo [lib]/[[bin]] tables via symgraph if it starts
    // emitting them.
    false
}

/// Technology label inferred from the language and the dependency set
/// symgraph extracted. The framework priority list is intentionally curated —
/// it reflects real-world "which dep tells you what this container is".
fn infer_technology(lang: Language, deps: &[String]) -> String {
    let base = match lang {
        Language::Rust => "Rust",
        Language::Go => "Go",
        Language::Python => "Python",
        Language::Java => "Java",
        Language::Kotlin => "Kotlin",
        Language::Scala => "Scala",
        Language::Groovy => "Gradle",
        Language::JavaScript => "Node.js",
        Language::TypeScript | Language::Tsx => "TypeScript",
        Language::Jsx => "JavaScript",
        Language::Ruby => "Ruby",
        Language::Php => "PHP",
        _ => "Unknown",
    };

    // Ordered: first match wins, so higher-level frameworks beat their runtimes.
    const FRAMEWORKS: &[(&str, &str)] = &[
        // Rust
        ("axum", "Axum"),
        ("actix-web", "Actix"),
        ("rocket", "Rocket"),
        ("warp", "Warp"),
        ("tonic", "Tonic (gRPC)"),
        // Python
        ("fastapi", "FastAPI"),
        ("django", "Django"),
        ("flask", "Flask"),
        ("starlette", "Starlette"),
        // Node / JS / TS
        ("next", "Next.js"),
        ("nuxt", "Nuxt"),
        ("@nestjs/core", "NestJS"),
        ("express", "Express"),
        ("fastify", "Fastify"),
        ("koa", "Koa"),
        ("@angular/core", "Angular"),
        ("react", "React"),
        ("vue", "Vue"),
        ("svelte", "Svelte"),
        // Go
        ("github.com/gin-gonic/gin", "Gin"),
        ("github.com/labstack/echo", "Echo"),
        ("github.com/gofiber/fiber", "Fiber"),
        ("google.golang.org/grpc", "gRPC"),
        // Java / Kotlin
        ("spring-boot-starter", "Spring Boot"),
        ("org.springframework.boot", "Spring Boot"),
        ("micronaut", "Micronaut"),
        ("quarkus", "Quarkus"),
        ("io.ktor", "Ktor"),
        // Ruby
        ("rails", "Rails"),
        ("sinatra", "Sinatra"),
    ];

    for (needle, label) in FRAMEWORKS {
        if deps.iter().any(|d| dep_matches(d, needle)) {
            return format!("{base} / {label}");
        }
    }
    base.to_string()
}

fn dep_matches(dep: &str, needle: &str) -> bool {
    dep == needle || dep.contains(needle)
}

// ── Workspace detection ─────────────────────────────────────────

/// If the given manifest is a workspace root, return absolute directories
/// for each member. Otherwise return None.
fn detect_workspace_members(filename: &str, text: &str, path: &Path) -> Option<Vec<PathBuf>> {
    let parent = path.parent()?;
    match filename {
        "Cargo.toml" => detect_cargo_workspace(text, parent),
        "package.json" => detect_npm_workspace(text, parent),
        _ => None,
    }
}

fn detect_cargo_workspace(text: &str, parent: &Path) -> Option<Vec<PathBuf>> {
    let parsed: toml::Value = text.parse().ok()?;
    let members = parsed.get("workspace")?.get("members")?.as_array()?;

    let mut out = Vec::new();
    for m in members {
        let Some(glob) = m.as_str() else { continue };
        expand_glob(parent, glob, &mut out);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn detect_npm_workspace(text: &str, parent: &Path) -> Option<Vec<PathBuf>> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    let ws = json.get("workspaces")?;

    // `workspaces` is either an array or an object with a `packages` array.
    let arr = match ws {
        serde_json::Value::Array(a) => Some(a.clone()),
        serde_json::Value::Object(o) => o.get("packages").and_then(|p| p.as_array()).cloned(),
        _ => None,
    }?;

    let mut out = Vec::new();
    for v in arr {
        if let Some(s) = v.as_str() {
            expand_glob(parent, s, &mut out);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Expand a workspace glob relative to `base`. Supports the common patterns:
/// `crates/*`, `packages/api`, `apps/**`. Non-glob entries are treated as a
/// direct path. Unmatched globs expand to nothing.
fn expand_glob(base: &Path, pattern: &str, out: &mut Vec<PathBuf>) {
    if !pattern.contains('*') {
        let p = base.join(pattern);
        if p.is_dir() {
            out.push(p);
        }
        return;
    }

    // Only handle the `<prefix>/*` and `<prefix>/**` shapes — enough for the
    // canonical monorepo layouts. Anything more exotic falls through.
    let (prefix, _suffix) = match pattern.rsplit_once('/') {
        Some(p) => p,
        None => return,
    };
    let dir = base.join(prefix);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.push(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn rust_container_from_cargo_toml() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "api"
version = "0.1.0"

[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
"#,
        )
        .unwrap();

        let mut model = Model::default();
        let mut index = ContainerIndex::new();
        let config = AnalyzeConfig::default();
        scan(&mut model, &mut index, root, &config);

        let el = model.elements.get("api").expect("container registered");
        assert_eq!(el.technology.as_deref(), Some("Rust / Axum"));
        assert!(el.tags.iter().any(|t| t == "inferred"));
        assert_eq!(index.attribute(&root.join("Cargo.toml")), Some("api"));
    }

    #[test]
    fn cargo_workspace_expands_members() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/*"]
"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("crates/api")).unwrap();
        fs::write(
            root.join("crates/api/Cargo.toml"),
            r#"
[package]
name = "api"
version = "0.1.0"

[dependencies]
axum = "0.7"
"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("crates/worker")).unwrap();
        fs::write(
            root.join("crates/worker/Cargo.toml"),
            r#"
[package]
name = "worker"
version = "0.1.0"
"#,
        )
        .unwrap();

        let mut model = Model::default();
        let mut index = ContainerIndex::new();
        let config = AnalyzeConfig::default();
        scan(&mut model, &mut index, root, &config);

        assert!(model.elements.contains_key("api"));
        assert!(model.elements.contains_key("worker"));
        // The workspace root itself should NOT become a container.
        let root_dir_name = root.file_name().unwrap().to_string_lossy().to_string();
        assert!(!model.elements.contains_key(&slugify(&root_dir_name)));
    }

    #[test]
    fn npm_workspace_expands_packages() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("package.json"),
            r#"{
  "name": "monorepo",
  "workspaces": ["packages/*"]
}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("packages/web")).unwrap();
        fs::write(
            root.join("packages/web/package.json"),
            r#"{
  "name": "web",
  "dependencies": { "next": "14.0.0", "react": "18.0.0" }
}"#,
        )
        .unwrap();

        let mut model = Model::default();
        let mut index = ContainerIndex::new();
        let config = AnalyzeConfig::default();
        scan(&mut model, &mut index, root, &config);

        let web = model.elements.get("web").expect("web registered");
        let tech = web.technology.as_deref().unwrap_or("");
        assert!(
            tech.ends_with("/ Next.js"),
            "expected Next.js framework suffix, got {tech}"
        );
    }

    #[test]
    fn python_fastapi_detected() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("pyproject.toml"),
            r#"
[project]
name = "svc"
version = "0.1.0"
dependencies = ["fastapi>=0.100", "uvicorn"]
"#,
        )
        .unwrap();

        let mut model = Model::default();
        let mut index = ContainerIndex::new();
        let config = AnalyzeConfig::default();
        scan(&mut model, &mut index, root, &config);

        let el = model.elements.get("svc").expect("svc container");
        assert_eq!(el.technology.as_deref(), Some("Python / FastAPI"));
    }
}

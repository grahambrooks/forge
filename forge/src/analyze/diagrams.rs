//! Diagrams scanner — pull elements and relationships out of PlantUML and
//! Mermaid diagrams that already live in the codebase.
//!
//! Looks at every standalone diagram file (`.puml`, `.plantuml`, `.iuml`,
//! `.mmd`, `.mermaid`) plus mermaid blocks fenced inside markdown
//! (` ```mermaid … ``` `). Each diagram is parsed via the existing
//! `crate::import::import` and merged into the Model with provenance
//! tags so `analyze --merge` can refresh it idempotently.
//!
//! Element ids from the imported model are namespaced by a slug of the
//! source file path, so two diagrams that both define `customer` don't
//! collide and both get represented.

use std::collections::HashMap;
use std::path::Path;

use super::iter_source_files;

use crate::import;
use crate::model::*;

use super::provenance::mark_inferred;
use super::{slugify, AnalyzeConfig};

const DIAGRAM_EXTS: &[&str] = &["puml", "plantuml", "iuml", "mmd", "mermaid"];
const SCANNER_NAME: &str = "diagrams";

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in iter_source_files(root, config, Some(10)) {
        let path = entry.path();

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());

        if let Some(ext) = ext.as_deref() {
            if DIAGRAM_EXTS.contains(&ext) {
                if let Ok(text) = std::fs::read_to_string(path) {
                    import_text_into_model(model, &text, path);
                }
                continue;
            }
            if ext == "md" || ext == "markdown" {
                if let Ok(text) = std::fs::read_to_string(path) {
                    for (idx, block) in extract_mermaid_blocks(&text).into_iter().enumerate() {
                        import_text_into_model(model, &block, &mark_block(path, idx));
                    }
                }
            }
        }
    }
}

/// Parse one diagram and merge its content into `model`. Element ids are
/// prefixed with a slug derived from `source` so multiple diagrams can
/// coexist without colliding on common names like `customer` or `db`.
fn import_text_into_model(model: &mut Model, text: &str, source: &Path) {
    let imported = match import::import(text) {
        Ok(m) => m,
        Err(_) => return,
    };

    let prefix = file_prefix(source);
    let mut id_map: HashMap<String, String> = HashMap::new();

    for (orig_id, mut el) in imported.elements {
        let new_id = format!("{prefix}-{}", slugify(&orig_id));
        id_map.insert(orig_id.clone(), new_id.clone());
        el.id = new_id.clone();
        // Imports may have inferred a parent id that we just rewrote.
        // Defer the parent rewrite until every id is known so the order
        // of insertion doesn't matter.
        mark_inferred(&mut el, SCANNER_NAME, Some(source));

        if model.elements.contains_key(&new_id) {
            continue;
        }
        model.elements.insert(new_id, el);
    }

    // Second pass: rewrite parent and children references to the new ids.
    for (orig_id, new_id) in &id_map {
        if let Some(el) = model.elements.get_mut(new_id) {
            if let Some(p) = el.parent.as_ref() {
                if let Some(mapped) = id_map.get(p) {
                    el.parent = Some(mapped.clone());
                }
            }
            el.children = el
                .children
                .iter()
                .map(|c| id_map.get(c).cloned().unwrap_or_else(|| c.clone()))
                .collect();
            // Suppress an unused warning on `orig_id` in builds where the
            // borrow checker happens to elide it.
            let _ = orig_id;
        }
    }

    for rel in imported.relationships {
        let frm = id_map.get(&rel.frm).cloned().unwrap_or(rel.frm);
        let to = id_map.get(&rel.to).cloned().unwrap_or(rel.to);
        if !model.elements.contains_key(&frm) || !model.elements.contains_key(&to) {
            continue;
        }
        let exists = model
            .relationships
            .iter()
            .any(|r| r.frm == frm && r.to == to && r.label == rel.label);
        if exists {
            continue;
        }
        model.add_relationship(Relationship {
            frm,
            to,
            label: rel.label,
            technology: rel.technology,
            order: rel.order,
        });
    }
}

/// Pull every ```mermaid``` fenced block out of a markdown document.
fn extract_mermaid_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(buf) = current.as_mut() {
            if trimmed.starts_with("```") {
                blocks.push(std::mem::take(buf));
                current = None;
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        } else if trimmed.starts_with("```mermaid") {
            current = Some(String::new());
        }
    }
    blocks
}

/// Slug used to namespace element ids for one source file. Falls back to
/// "diagram" if the path has no useful name.
fn file_prefix(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("diagram");
    let slug = slugify(stem);
    if slug.is_empty() {
        "diagram".into()
    } else {
        slug
    }
}

/// Synthetic source path for one mermaid block extracted from a markdown
/// file. Lets multiple blocks in one document keep distinct id namespaces
/// and distinct `forge:source` values.
fn mark_block(path: &Path, idx: usize) -> std::path::PathBuf {
    let mut p = path.to_path_buf();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc")
        .to_string();
    p.set_file_name(format!("{stem}#mermaid-{idx}.mmd"));
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn imports_plantuml_c4_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("context.puml");
        fs::write(
            &path,
            "@startuml\n\
             Person(customer, \"Customer\", \"End user\")\n\
             System(payments, \"Payments\", \"Card processing\")\n\
             Rel(customer, payments, \"makes payments\")\n\
             @enduml\n",
        )
        .unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig {
            paths: vec![dir.path().into()],
            ..Default::default()
        };
        scan(&mut model, dir.path(), &cfg);

        assert!(model
            .elements
            .values()
            .any(|el| el.name == "Customer" && el.kind == ElementKind::Person));
        assert!(model
            .elements
            .values()
            .any(|el| el.name == "Payments" && el.kind == ElementKind::System));
        assert_eq!(model.relationships.len(), 1);
        assert_eq!(model.relationships[0].label, "makes payments");
    }

    #[test]
    fn imports_mermaid_flowchart_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("flow.mmd");
        fs::write(
            &path,
            "flowchart LR\n\
             A[Web] -->|http| B[API]\n\
             B --> C[Database]\n",
        )
        .unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig::default();
        scan(&mut model, dir.path(), &cfg);

        assert_eq!(model.elements.len(), 3);
        assert_eq!(model.relationships.len(), 2);
        assert!(model.relationships.iter().any(|r| r.label == "http"));
    }

    #[test]
    fn imports_mermaid_block_inside_markdown() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("README.md");
        fs::write(
            &path,
            "# Architecture\n\
             \n\
             ```mermaid\n\
             flowchart TD\n\
             A[Client] --> B[Service]\n\
             ```\n\
             \n\
             More prose.\n",
        )
        .unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig::default();
        scan(&mut model, dir.path(), &cfg);

        assert_eq!(model.elements.len(), 2);
        assert!(model.elements.values().any(|el| el.name == "Client"));
    }

    #[test]
    fn namespaces_ids_per_source_file() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("a.mmd"),
            "flowchart LR\nA[Service] --> B[DB]\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("b.mmd"),
            "flowchart LR\nA[Worker] --> B[Queue]\n",
        )
        .unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig::default();
        scan(&mut model, dir.path(), &cfg);

        // Both diagrams' nodes live in the model — the per-file slug
        // prefix prevents the second `A` from clobbering the first.
        let names: Vec<_> = model.elements.values().map(|el| el.name.clone()).collect();
        assert!(names.contains(&"Service".to_string()));
        assert!(names.contains(&"Worker".to_string()));
        assert_eq!(model.elements.len(), 4);
    }

    #[test]
    fn marks_imported_elements_inferred_with_source() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ctx.puml");
        fs::write(&path, "@startuml\nPerson(u, \"User\", \"\")\n@enduml\n").unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig::default();
        scan(&mut model, dir.path(), &cfg);

        let el = model.elements.values().next().unwrap();
        assert!(el.tags.iter().any(|t| t == "inferred"));
        assert!(el.tags.iter().any(|t| t == "inferred:diagrams"));
        assert!(el.properties.contains_key("forge:source"));
    }

    #[test]
    fn skips_unparseable_files() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("noise.puml"),
            "this is not actually plantuml at all\n",
        )
        .unwrap();

        let mut model = Model::default();
        let cfg = AnalyzeConfig::default();
        scan(&mut model, dir.path(), &cfg);
        assert!(model.elements.is_empty());
    }
}

//! `forge export` and `forge import` — JSON/YAML out, PlantUML/Mermaid in.

use std::fs;
use std::path::Path;

use crate::{export, import};

use super::util::{die, load_model, write_or_stdout};

pub fn cmd_export(source: &Path, format: &str, out: Option<&Path>) {
    let model = load_model(source);
    let output = match format {
        "json" => export::to_json(&model),
        "yaml" | "yml" => export::to_yaml(&model),
        _ => die("--format must be 'json' or 'yaml'"),
    };
    write_or_stdout(out, &output);
}

pub fn cmd_import(source: &Path, out: Option<&Path>) {
    let text =
        fs::read_to_string(source).unwrap_or_else(|e| die(&format!("{}: {}", source.display(), e)));
    let forge_text =
        import::import_to_forge(&text).unwrap_or_else(|e| die(&format!("import: {}", e)));
    write_or_stdout(out, &forge_text);
}

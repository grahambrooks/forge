//! `forge analyze` — scan a codebase and emit a `.forge` model.

use std::fs;
use std::path::{Path, PathBuf};

use crate::analyze;

use super::util::{die, load_model};

pub fn cmd_analyze(
    paths: Vec<PathBuf>,
    out: &Path,
    scanners: &str,
    exclude: Vec<String>,
    dry_run: bool,
    merge_into: Option<&Path>,
    plugins: Vec<String>,
) {
    let scanner_list: Vec<String> = scanners.split(',').map(|s| s.trim().to_string()).collect();
    let plugin_cmds: Vec<analyze::plugin::PluginCommand> = plugins
        .iter()
        .filter_map(|p| analyze::plugin::PluginCommand::from_cli(p))
        .collect();
    let mut config = analyze::AnalyzeConfig {
        paths,
        scanners: scanner_list,
        out: out.to_path_buf(),
        dry_run,
        plugins: plugin_cmds,
        ..Default::default()
    };
    config.exclude.extend(exclude);

    eprintln!("Scanning...");
    let fresh = analyze::analyze(&config);
    eprintln!("  Elements: {}", fresh.elements.len());
    eprintln!("  Relationships: {}", fresh.relationships.len());

    let model = if let Some(existing_path) = merge_into {
        eprintln!("Merging into {}...", existing_path.display());
        let mut existing = load_model(existing_path);
        let before_elements = existing.elements.len();
        let before_rels = existing.relationships.len();
        analyze::merge::merge(&mut existing, fresh);
        eprintln!(
            "  Merged: {} elements ({} before), {} relationships ({} before)",
            existing.elements.len(),
            before_elements,
            existing.relationships.len(),
            before_rels
        );
        existing
    } else {
        fresh
    };

    let forge_text = analyze::emit::emit(&model);
    if dry_run {
        println!("{}", forge_text);
    } else {
        fs::write(out, &forge_text)
            .unwrap_or_else(|e| die(&format!("writing {}: {}", out.display(), e)));
        eprintln!("  Wrote: {}", out.display());
    }
    eprintln!("Done.");
}

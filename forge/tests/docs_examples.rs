//! Every ```` ```forge ```` block under `docs/` must parse with the current
//! grammar, so the next keyword change can't strand the examples (#45).
//!
//! A block is checked according to how it starts:
//!
//! * `forge "…"` — a complete model, parsed as it stands;
//! * a top-level block keyword (`model`, `views`, `process`, …) — parsed
//!   inside a `forge "docs" { … }` envelope;
//! * a view kind (`…-view`) — parsed inside `forge "docs" { views { … } }`;
//! * anything else — a fragment of some inner block (a few elements, a stage),
//!   which has no context to parse in, so it is skipped.
//!
//! A block that is deliberately not valid DSL (a schematic with `...`, a
//! proposal that never shipped) opts out with `<!-- forge-docs-test: skip -->`
//! on the line before its fence. The "Before" examples in
//! docs/reference/grammar.md are inline code, not ```` ```forge ```` blocks,
//! so they are never picked up.

use std::path::{Path, PathBuf};
use std::process::Command;

const SKIP_MARKER: &str = "<!-- forge-docs-test: skip -->";

const TOP_LEVEL: &[&str] = &[
    "description",
    "model",
    "process",
    "deployment",
    "tech-stack",
    "data-model",
    "trust-boundaries",
    "teams",
    "apis",
    "event-flows",
    "env-config",
    "slos",
    "dependencies",
    "views",
    "docs",
];

struct Block {
    file: PathBuf,
    line: usize,
    text: String,
    skipped: bool,
}

fn docs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs")
}

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

fn forge_blocks(file: &Path) -> Vec<Block> {
    let text = std::fs::read_to_string(file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "```forge" {
            let skipped = lines[..i]
                .iter()
                .rev()
                .find(|l| !l.trim().is_empty())
                .is_some_and(|l| l.trim() == SKIP_MARKER);
            let start = i + 1;
            let end = (start..lines.len())
                .find(|&j| lines[j].trim() == "```")
                .unwrap_or_else(|| panic!("{}:{}: unclosed ```forge block", file.display(), i + 1));
            blocks.push(Block {
                file: file.to_path_buf(),
                line: start + 1,
                text: lines[start..end].join("\n"),
                skipped,
            });
            i = end;
        }
        i += 1;
    }
    blocks
}

fn all_blocks() -> Vec<Block> {
    let mut files = Vec::new();
    markdown_files(&docs_dir(), &mut files);
    files.iter().flat_map(|f| forge_blocks(f)).collect()
}

/// The block as a complete model, or `None` for an inner fragment.
fn as_model(text: &str) -> Option<String> {
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("//"))?
        .split_whitespace()
        .next()?;
    if first == "forge" {
        Some(text.to_string())
    } else if TOP_LEVEL.contains(&first) {
        Some(format!("forge \"docs\" {{\n{text}\n}}\n"))
    } else if first.ends_with("-view") {
        Some(format!("forge \"docs\" {{ views {{\n{text}\n}} }}\n"))
    } else {
        None
    }
}

#[test]
fn forge_examples_in_the_docs_parse() {
    let blocks = all_blocks();
    let mut checked = 0;
    let mut failures = Vec::new();
    for block in blocks.iter().filter(|b| !b.skipped) {
        let Some(model) = as_model(&block.text) else {
            continue;
        };
        checked += 1;
        let base = block.file.parent().unwrap();
        if let Err(e) = forge_dsl::parse(&model, base) {
            let rel = block.file.strip_prefix(docs_dir()).unwrap_or(&block.file);
            failures.push(format!("docs/{}:{}: {}", rel.display(), block.line, e.msg));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {checked} docs examples fail to parse:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // Guard against the extraction silently finding nothing.
    assert!(checked >= 20, "only {checked} docs examples were checked");
}

#[test]
fn the_first_model_walkthrough_checks_clean() {
    // docs/user-guide/first-model.md promises "zero violations".
    let file = docs_dir().join("user-guide/first-model.md");
    let block = forge_blocks(&file)
        .into_iter()
        .find(|b| b.text.trim_start().starts_with("forge "))
        .expect("first-model.md has a complete example");
    let dir = std::env::temp_dir().join(format!("forge-first-model-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("architecture.forge");
    std::fs::write(&source, &block.text).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["check", "--source"])
        .arg(&source)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && format!("{stdout}{stderr}").contains("No issues found"),
        "status {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
}

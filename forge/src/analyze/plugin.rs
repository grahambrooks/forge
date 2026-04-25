//! External-process plugin runner for `forge analyze`.
//!
//! A plugin is any executable that speaks newline-delimited JSON over stdio:
//!
//!   forge ──init──▶ plugin
//!   forge ◀──ready (capabilities) ── plugin
//!   forge ──analyze_file (path)──▶ plugin
//!   forge ◀──patch (additions) ── plugin
//!   …repeat…
//!   forge ──finalize──▶ plugin   (optional, plugin opts in via `wants_finalize`)
//!   forge ◀──patch ── plugin
//!
//! Patch responses are *additive*: elements, relationships, property overlays,
//! and tag additions. Forge auto-stamps every contributed element with the
//! existing `inferred` / `inferred:plugin:<name>` provenance tags so
//! `analyze::merge` already knows how to refresh plugin output on re-runs.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::model::{Element, ElementKind, Model, Relationship};

use super::provenance::mark_inferred;
use super::AnalyzeConfig;

const PROTOCOL: u32 = 1;

/// One plugin invocation as configured by the user (CLI or config file).
#[derive(Debug, Clone)]
pub struct PluginCommand {
    /// argv — first element is the executable, rest are arguments.
    pub argv: Vec<String>,
    /// Free-form options passed verbatim to the plugin in the `init` message.
    pub options: serde_json::Value,
}

impl PluginCommand {
    /// Parse `--plugin "tsx ./plugin.ts"`-style values. Whitespace splits
    /// argv; quoting is not supported (use a wrapper script if you need it).
    pub fn from_cli(value: &str) -> Option<Self> {
        let argv: Vec<String> = value.split_whitespace().map(String::from).collect();
        if argv.is_empty() {
            None
        } else {
            Some(Self {
                argv,
                options: serde_json::Value::Null,
            })
        }
    }
}

// ─── Wire format ────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Outbound<'a> {
    Init {
        protocol: u32,
        forge_version: &'a str,
        scan_root: &'a str,
        options: &'a serde_json::Value,
    },
    AnalyzeFile {
        protocol: u32,
        path: &'a str,
        relative_path: &'a str,
    },
    Finalize {
        protocol: u32,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Inbound {
    Ready(ReadyMsg),
    Patch(PatchMsg),
    Error(ErrorMsg),
}

#[derive(Deserialize)]
struct ReadyMsg {
    name: String,
    #[serde(default)]
    #[allow(dead_code)]
    version: String,
    #[serde(rename = "match")]
    matchers: Vec<MatcherSpec>,
    #[serde(default)]
    wants_finalize: bool,
}

#[derive(Deserialize)]
struct MatcherSpec {
    kind: String,
    pattern: String,
}

#[derive(Deserialize, Default)]
struct PatchMsg {
    #[serde(default)]
    elements: Vec<PluginElement>,
    #[serde(default)]
    relationships: Vec<PluginRelationship>,
    #[serde(default)]
    property_overlays: Vec<PropertyOverlay>,
    #[serde(default)]
    tag_additions: Vec<TagAddition>,
}

#[derive(Deserialize)]
struct PluginElement {
    id: String,
    kind: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    technology: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    properties: HashMap<String, String>,
}

#[derive(Deserialize)]
struct PluginRelationship {
    from: String,
    to: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    technology: Option<String>,
}

#[derive(Deserialize)]
struct PropertyOverlay {
    id: String,
    properties: HashMap<String, String>,
}

#[derive(Deserialize)]
struct TagAddition {
    id: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct ErrorMsg {
    #[serde(default)]
    message: String,
}

// ─── Matchers ───────────────────────────────────────────────────────

enum Matcher {
    Filename(String),
    Regex(regex::Regex),
    Glob(regex::Regex),
}

impl Matcher {
    fn compile(spec: &MatcherSpec) -> Result<Self, String> {
        match spec.kind.as_str() {
            "filename" => Ok(Matcher::Filename(spec.pattern.clone())),
            "regex" => regex::Regex::new(&spec.pattern)
                .map(Matcher::Regex)
                .map_err(|e| format!("bad regex {:?}: {e}", spec.pattern)),
            "glob" => regex::Regex::new(&glob_to_regex(&spec.pattern))
                .map(Matcher::Glob)
                .map_err(|e| format!("bad glob {:?}: {e}", spec.pattern)),
            other => Err(format!("unknown matcher kind {:?}", other)),
        }
    }

    fn matches(&self, relative_path: &str) -> bool {
        match self {
            Matcher::Filename(name) => Path::new(relative_path)
                .file_name()
                .map(|f| f == name.as_str())
                .unwrap_or(false),
            Matcher::Regex(re) | Matcher::Glob(re) => re.is_match(relative_path),
        }
    }
}

/// Translate a `**/foo.{yaml,yml}`-style glob to an anchored regex.
fn glob_to_regex(pat: &str) -> String {
    let mut out = String::from("^");
    let mut chars = pat.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if chars.peek() == Some(&'/') {
                        chars.next();
                    }
                    out.push_str(".*");
                } else {
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            '{' => {
                out.push('(');
                let mut first = true;
                for c in chars.by_ref() {
                    match c {
                        '}' => break,
                        ',' => {
                            out.push('|');
                            first = true;
                        }
                        c => {
                            if !first {
                                // already inside an alternative
                            }
                            first = false;
                            push_escaped(&mut out, c);
                        }
                    }
                }
                out.push(')');
            }
            c => push_escaped(&mut out, c),
        }
    }
    out.push('$');
    out
}

fn push_escaped(out: &mut String, c: char) {
    if matches!(
        c,
        '.' | '+' | '(' | ')' | '|' | '^' | '$' | '[' | ']' | '\\'
    ) {
        out.push('\\');
    }
    out.push(c);
}

// ─── Runner ─────────────────────────────────────────────────────────

/// Run every configured plugin against `scan_root`, applying patches into
/// `model`. Failures are reported on stderr; one bad plugin never aborts the
/// rest of the analyze run.
pub fn run(plugins: &[PluginCommand], scan_root: &Path, config: &AnalyzeConfig, model: &mut Model) {
    for cmd in plugins {
        if let Err(e) = run_one(cmd, scan_root, config, model) {
            eprintln!("  Plugin {:?} failed: {}", cmd.argv.join(" "), e);
        }
    }
}

fn run_one(
    cmd: &PluginCommand,
    scan_root: &Path,
    config: &AnalyzeConfig,
    model: &mut Model,
) -> Result<(), String> {
    let mut child = Command::new(&cmd.argv[0])
        .args(&cmd.argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("spawn: {e}"))?;

    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut io = PluginIo {
        stdin,
        reader: BufReader::new(stdout),
        line: String::new(),
    };

    io.send(&Outbound::Init {
        protocol: PROTOCOL,
        forge_version: env!("CARGO_PKG_VERSION"),
        scan_root: &scan_root.display().to_string(),
        options: &cmd.options,
    })?;

    let ready = match io.recv()? {
        Inbound::Ready(r) => r,
        Inbound::Error(e) => return Err(format!("plugin error during init: {}", e.message)),
        _ => return Err("expected ready, got patch".into()),
    };

    let plugin_name = ready.name.clone();
    let matchers: Vec<Matcher> = ready
        .matchers
        .iter()
        .map(Matcher::compile)
        .collect::<Result<_, _>>()?;

    eprintln!(
        "  Plugin '{}': {} matchers, finalize={}",
        plugin_name,
        matchers.len(),
        ready.wants_finalize
    );

    let mut files_processed = 0usize;
    let walker = WalkDir::new(scan_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    let mut patches_to_apply: Vec<(PatchMsg, Option<PathBuf>)> = Vec::new();

    for entry in walker {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let relative = path.strip_prefix(scan_root).unwrap_or(path);
        let rel_str = relative.to_string_lossy().replace('\\', "/");

        if !matchers.iter().any(|m| m.matches(&rel_str)) {
            continue;
        }

        let abs_str = path.display().to_string();
        io.send(&Outbound::AnalyzeFile {
            protocol: PROTOCOL,
            path: &abs_str,
            relative_path: &rel_str,
        })?;

        match io.recv()? {
            Inbound::Patch(p) => {
                patches_to_apply.push((p, Some(path.to_path_buf())));
                files_processed += 1;
            }
            Inbound::Error(e) => {
                eprintln!("    [{}] error on {}: {}", plugin_name, rel_str, e.message);
            }
            Inbound::Ready(_) => {
                return Err("unexpected second ready".into());
            }
        }
    }

    if ready.wants_finalize {
        io.send(&Outbound::Finalize { protocol: PROTOCOL })?;
        match io.recv()? {
            Inbound::Patch(p) => patches_to_apply.push((p, None)),
            Inbound::Error(e) => {
                eprintln!("    [{}] finalize error: {}", plugin_name, e.message);
            }
            Inbound::Ready(_) => return Err("unexpected ready in finalize".into()),
        }
    }

    drop(io);
    let _ = child.wait();

    for (patch, source) in patches_to_apply {
        apply_patch(model, patch, &plugin_name, source.as_deref());
    }
    eprintln!(
        "  Plugin '{}': {} files processed",
        plugin_name, files_processed
    );

    Ok(())
}

struct PluginIo {
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    line: String,
}

impl PluginIo {
    fn send<T: Serialize>(&mut self, msg: &T) -> Result<(), String> {
        let s = serde_json::to_string(msg).map_err(|e| format!("encode: {e}"))?;
        self.stdin
            .write_all(s.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .map_err(|e| format!("write: {e}"))
    }

    fn recv(&mut self) -> Result<Inbound, String> {
        self.line.clear();
        let n = self
            .reader
            .read_line(&mut self.line)
            .map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            return Err("plugin closed stdout unexpectedly".into());
        }
        serde_json::from_str(self.line.trim())
            .map_err(|e| format!("decode {:?}: {e}", self.line.trim()))
    }
}

// ─── Patch application ─────────────────────────────────────────────

fn apply_patch(model: &mut Model, patch: PatchMsg, plugin_name: &str, source: Option<&Path>) {
    let scanner = format!("plugin:{plugin_name}");

    for pe in patch.elements {
        let kind = match parse_kind(&pe.kind) {
            Some(k) => k,
            None => {
                eprintln!("    [{}] unknown kind {:?}, skipping", plugin_name, pe.kind);
                continue;
            }
        };

        if model.elements.contains_key(&pe.id) {
            // User content (or another scanner) already owns this id.
            // Existing merge rule: user wins.
            continue;
        }

        let mut el = Element::new(pe.id.clone(), kind, pe.name);
        el.description = pe.description;
        el.technology = pe.technology;
        el.parent = pe.parent;
        el.tags = pe.tags;
        el.properties = pe.properties;
        mark_inferred(&mut el, &scanner, source);
        model.elements.insert(pe.id, el);
    }

    for pr in patch.relationships {
        if !model.elements.contains_key(&pr.frm_or_from()) || !model.elements.contains_key(&pr.to) {
            continue;
        }
        let already = model
            .relationships
            .iter()
            .any(|r| r.frm == pr.frm_or_from() && r.to == pr.to);
        if already {
            continue;
        }
        model.relationships.push(Relationship {
            frm: pr.frm_or_from(),
            to: pr.to,
            label: pr.label.unwrap_or_default(),
            technology: pr.technology,
            order: None,
        });
    }

    for ov in patch.property_overlays {
        if let Some(el) = model.elements.get_mut(&ov.id) {
            for (k, v) in ov.properties {
                if k.contains(':') {
                    el.properties.insert(k, v);
                } else {
                    eprintln!(
                        "    [{}] refused overlay {:?} on {}: keys must contain ':'",
                        plugin_name, k, ov.id
                    );
                }
            }
        }
    }

    for ta in patch.tag_additions {
        if let Some(el) = model.elements.get_mut(&ta.id) {
            for t in ta.tags {
                if !el.tags.iter().any(|x| x == &t) {
                    el.tags.push(t);
                }
            }
        }
    }
}

impl PluginRelationship {
    fn frm_or_from(&self) -> String {
        self.from.clone()
    }
}

fn parse_kind(s: &str) -> Option<ElementKind> {
    match s {
        "Person" => Some(ElementKind::Person),
        "System" => Some(ElementKind::System),
        "Container" => Some(ElementKind::Container),
        "Component" => Some(ElementKind::Component),
        "Repository" => Some(ElementKind::Repository),
        "Branch" => Some(ElementKind::Branch),
        "Pipeline" => Some(ElementKind::Pipeline),
        "Stage" => Some(ElementKind::Stage),
        "Environment" => Some(ElementKind::Environment),
        "Gate" => Some(ElementKind::Gate),
        "Step" => Some(ElementKind::Step),
        "Artifact" => Some(ElementKind::Artifact),
        "DeploymentNode" => Some(ElementKind::DeploymentNode),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_translates_doublestar() {
        let re = regex::Regex::new(&glob_to_regex("**/openapi.{yaml,yml,json}")).unwrap();
        assert!(re.is_match("svc/api/openapi.yaml"));
        assert!(re.is_match("openapi.json"));
        assert!(!re.is_match("svc/api/openapi.txt"));
    }

    #[test]
    fn glob_singlestar_does_not_cross_slash() {
        let re = regex::Regex::new(&glob_to_regex("*.json")).unwrap();
        assert!(re.is_match("a.json"));
        assert!(!re.is_match("a/b.json"));
    }

    #[test]
    fn filename_matcher_strips_directories() {
        let m = Matcher::Filename("openapi.yaml".into());
        assert!(m.matches("svc/api/openapi.yaml"));
        assert!(!m.matches("svc/api/openapi.yml"));
    }

    #[test]
    fn apply_patch_stamps_inferred_provenance() {
        let mut model = Model::default();
        let patch = PatchMsg {
            elements: vec![PluginElement {
                id: "svc.api".into(),
                kind: "Container".into(),
                name: "Payments API".into(),
                description: None,
                technology: Some("OpenAPI".into()),
                parent: None,
                tags: vec![],
                properties: HashMap::new(),
            }],
            ..Default::default()
        };
        apply_patch(
            &mut model,
            patch,
            "openapi",
            Some(Path::new("svc/openapi.yaml")),
        );
        let el = model.elements.get("svc.api").unwrap();
        assert!(el.tags.iter().any(|t| t == "inferred"));
        assert!(el.tags.iter().any(|t| t == "inferred:plugin:openapi"));
        assert_eq!(
            el.properties.get("forge:source").map(String::as_str),
            Some("svc/openapi.yaml")
        );
    }

    #[test]
    fn apply_patch_user_wins_on_id_collision() {
        let mut model = Model::default();
        model.elements.insert(
            "svc.api".into(),
            Element::new("svc.api", ElementKind::Container, "User-named"),
        );
        let patch = PatchMsg {
            elements: vec![PluginElement {
                id: "svc.api".into(),
                kind: "Container".into(),
                name: "Plugin-named".into(),
                description: None,
                technology: None,
                parent: None,
                tags: vec![],
                properties: HashMap::new(),
            }],
            ..Default::default()
        };
        apply_patch(&mut model, patch, "openapi", None);
        assert_eq!(model.elements.get("svc.api").unwrap().name, "User-named");
    }

    #[test]
    fn property_overlay_requires_namespace() {
        let mut model = Model::default();
        model.elements.insert(
            "svc.api".into(),
            Element::new("svc.api", ElementKind::Container, "API"),
        );
        let mut props = HashMap::new();
        props.insert("name".into(), "hijacked".into()); // no colon → refused
        props.insert("openapi:routes".into(), "42".into());
        let patch = PatchMsg {
            property_overlays: vec![PropertyOverlay {
                id: "svc.api".into(),
                properties: props,
            }],
            ..Default::default()
        };
        apply_patch(&mut model, patch, "openapi", None);
        let el = model.elements.get("svc.api").unwrap();
        assert_eq!(el.name, "API");
        assert_eq!(
            el.properties.get("openapi:routes").map(String::as_str),
            Some("42")
        );
        assert!(!el.properties.contains_key("name"));
    }
}

//! Forge MCP server — JSON-RPC 2.0 over stdio.
//!
//! Implements the Model Context Protocol for AI agent integration.
//! Tools: forge_analyze, forge_reload, forge_overview, forge_list_views,
//! forge_query, forge_render, forge_check, forge_element_detail,
//! forge_search, forge_validate

use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::analyze;
use crate::check;
use crate::layout;
use crate::model::*;
use crate::parser;
use crate::render;

/// Holds the mutable state the MCP server exposes to clients.
///
/// The source path tracks which `.forge` file the in-memory model came
/// from. It is `None` when the server was started without a source and
/// has not yet run `forge_analyze` — in that case every "model" tool
/// returns a clear error pointing the client at `forge_analyze`.
struct ServerState {
    model: Model,
    source: Option<PathBuf>,
}

struct McpServer {
    state: RefCell<ServerState>,
}

impl McpServer {
    fn new(source: Option<PathBuf>) -> Result<Self, String> {
        let state = match source {
            Some(path) => {
                let model = load_model(&path)?;
                ServerState {
                    model,
                    source: Some(path),
                }
            }
            None => ServerState {
                model: Model::default(),
                source: None,
            },
        };
        Ok(Self {
            state: RefCell::new(state),
        })
    }

    fn handle_request(&self, msg: &Value) -> Option<Value> {
        let method = msg.get("method")?.as_str()?;
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => self.handle_initialize(),
            "initialized" => return None, // notification, no response
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&params),
            _ => Err(json!({"code": -32601, "message": format!("Method not found: {}", method)})),
        };

        Some(match result {
            Ok(res) => json!({"jsonrpc": "2.0", "id": id, "result": res}),
            Err(err) => json!({"jsonrpc": "2.0", "id": id, "error": err}),
        })
    }

    fn handle_initialize(&self) -> Result<Value, Value> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "forge",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Forge architecture model server. Use forge_analyze to infer a \
                model from a repository, forge_overview to see what's in the model, \
                forge_query/forge_search to explore elements, forge_element_detail for \
                drill-down, forge_render for SVG, forge_check for lint violations."
        }))
    }

    fn handle_tools_list(&self) -> Result<Value, Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "forge_analyze",
                    "description": "Analyze a codebase and load the inferred model into the server. Writes a .forge file alongside the source if `out` is given, otherwise just loads the model. Replaces any model currently held in memory.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Directory to analyze (default: current working directory)"},
                            "out": {"type": "string", "description": "Optional path to write the emitted .forge file (e.g. architecture.forge)"},
                            "scanners": {"type": "string", "description": "Comma-separated scanner list. Default: code,semantic,ci,docker,git,k8s,infra"},
                            "exclude": {"type": "array", "items": {"type": "string"}, "description": "Additional directory names to exclude"},
                            "merge": {"type": "boolean", "description": "If true, merge into the current model instead of replacing (preserves hand-authored content)"}
                        }
                    }
                },
                {
                    "name": "forge_reload",
                    "description": "Reload the model from the .forge file that was passed at startup, or the last file written via forge_analyze. Use this after an external edit to pick up changes.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "source": {"type": "string", "description": "Override the source path to reload from"}
                        }
                    }
                },
                {
                    "name": "forge_overview",
                    "description": "Summarise the loaded model — counts by element kind, top-level systems, view keys, and tech-stack size. Use this as the first call after loading a model.",
                    "inputSchema": {"type": "object", "properties": {}}
                },
                {
                    "name": "forge_list_views",
                    "description": "List every view in the model with its kind and optional title. Follow up with forge_render using one of the returned keys.",
                    "inputSchema": {"type": "object", "properties": {}}
                },
                {
                    "name": "forge_query",
                    "description": "Query the architecture model. List elements filtered by kind, tag, or name pattern.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "description": "Filter by element kind (container, system, person, component, pipeline, stage, branch, deploymentnode)"},
                            "tag": {"type": "string", "description": "Filter by tag (e.g. database, pci, inferred:docker)"},
                            "name": {"type": "string", "description": "Filter by name substring (case-insensitive)"}
                        }
                    }
                },
                {
                    "name": "forge_render",
                    "description": "Render a view to SVG. Returns the SVG string.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "view_key": {"type": "string", "description": "View key (e.g. SystemContext, Containers, Pipeline)"},
                            "style": {"type": "string", "description": "Rendering style: filled (default) or outline"}
                        },
                        "required": ["view_key"]
                    }
                },
                {
                    "name": "forge_check",
                    "description": "Run architectural lint rules. Returns violations.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "severity": {"type": "string", "description": "Minimum severity: error, warning, info"}
                        }
                    }
                },
                {
                    "name": "forge_element_detail",
                    "description": "Get full details for a specific element by ID, including children and relationships.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "Element ID (e.g. payments.api)"}
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "forge_search",
                    "description": "Search elements by name, description, or technology.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Search query"}
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "forge_validate",
                    "description": "Parse and validate a .forge code snippet.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "code": {"type": "string", "description": "Forge DSL code to validate"}
                        },
                        "required": ["code"]
                    }
                }
            ]
        }))
    }

    fn handle_tools_call(&self, params: &Value) -> Result<Value, Value> {
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = match name {
            "forge_analyze" => self.tool_analyze(&args),
            "forge_reload" => self.tool_reload(&args),
            "forge_overview" => self.tool_overview(),
            "forge_list_views" => self.tool_list_views(),
            "forge_query" => self.tool_query(&args),
            "forge_render" => self.tool_render(&args),
            "forge_check" => self.tool_check(&args),
            "forge_element_detail" => self.tool_element_detail(&args),
            "forge_search" => self.tool_search(&args),
            "forge_validate" => self.tool_validate(&args),
            _ => {
                return Err(json!({"code": -32602, "message": format!("Unknown tool: {}", name)}));
            }
        };

        Ok(json!({
            "content": [{"type": "text", "text": result}]
        }))
    }

    // ── Tool implementations ─────────────────────────────────────

    fn tool_analyze(&self, args: &Value) -> String {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        if !path.exists() {
            return json!({"error": format!("path does not exist: {}", path.display())})
                .to_string();
        }

        let scanners = args
            .get("scanners")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                vec![
                    "code".into(),
                    "semantic".into(),
                    "ci".into(),
                    "docker".into(),
                    "git".into(),
                    "k8s".into(),
                    "infra".into(),
                ]
            });

        let mut exclude: Vec<String> = Vec::new();
        if let Some(arr) = args.get("exclude").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    exclude.push(s.to_string());
                }
            }
        }

        let out_path = args.get("out").and_then(|v| v.as_str()).map(PathBuf::from);

        let mut config = analyze::AnalyzeConfig {
            paths: vec![path.clone()],
            scanners,
            out: out_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("forge.forge")),
            dry_run: out_path.is_none(),
            ..Default::default()
        };
        config.exclude.extend(exclude);

        let fresh = analyze::analyze(&config);

        let merge = args.get("merge").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut state = self.state.borrow_mut();
        if merge && !state.model.elements.is_empty() {
            analyze::merge::merge(&mut state.model, fresh);
        } else {
            state.model = fresh;
        }

        let mut write_status = json!(null);
        if let Some(out) = &out_path {
            let text = analyze::emit::emit(&state.model);
            match std::fs::write(out, text) {
                Ok(_) => {
                    state.source = Some(out.clone());
                    write_status = json!({"wrote": out.display().to_string()});
                }
                Err(e) => {
                    write_status = json!({"error": format!("write {}: {}", out.display(), e)});
                }
            }
        }

        let kinds = element_kind_counts(&state.model);
        serde_json::to_string_pretty(&json!({
            "analyzed": path.display().to_string(),
            "name": state.model.name,
            "elements": state.model.elements.len(),
            "relationships": state.model.relationships.len(),
            "views": state.model.views.len(),
            "kinds": kinds,
            "source": state.source.as_ref().map(|p| p.display().to_string()),
            "file": write_status,
        }))
        .unwrap_or_default()
    }

    fn tool_reload(&self, args: &Value) -> String {
        let override_path = args
            .get("source")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let mut state = self.state.borrow_mut();
        let path = match override_path.or_else(|| state.source.clone()) {
            Some(p) => p,
            None => {
                return json!({
                    "error": "no source to reload. Pass `source`, or call forge_analyze with an `out` path first."
                })
                .to_string();
            }
        };

        match load_model(&path) {
            Ok(m) => {
                state.model = m;
                state.source = Some(path.clone());
                serde_json::to_string_pretty(&json!({
                    "reloaded": path.display().to_string(),
                    "elements": state.model.elements.len(),
                    "relationships": state.model.relationships.len(),
                    "views": state.model.views.len(),
                }))
                .unwrap_or_default()
            }
            Err(e) => json!({"error": e}).to_string(),
        }
    }

    fn tool_overview(&self) -> String {
        let state = self.state.borrow();
        let model = &state.model;
        if model.elements.is_empty() && model.views.is_empty() {
            return json!({
                "empty": true,
                "hint": "No model loaded. Call forge_analyze to infer one from a repository, or pass --source at startup.",
            })
            .to_string();
        }

        let top_systems: Vec<Value> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::System && e.parent.is_none())
            .map(|e| json!({"id": e.id, "name": e.name}))
            .collect();

        let top_containers: Vec<Value> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Container && e.parent.is_none())
            .map(|e| json!({"id": e.id, "name": e.name, "technology": e.technology}))
            .collect();

        let views: Vec<Value> = model
            .views
            .iter()
            .map(|v| json!({"key": v.key, "kind": format!("{:?}", v.kind), "title": v.title}))
            .collect();

        serde_json::to_string_pretty(&json!({
            "name": model.name,
            "description": model.description,
            "source": state.source.as_ref().map(|p| p.display().to_string()),
            "counts": {
                "elements": model.elements.len(),
                "relationships": model.relationships.len(),
                "views": model.views.len(),
                "tech_categories": model.tech_stack.len(),
                "data_entities": model.data_entities.len(),
                "teams": model.teams.len(),
                "trust_boundaries": model.trust_boundaries.len(),
            },
            "by_kind": element_kind_counts(model),
            "top_level_systems": top_systems,
            "top_level_containers": top_containers,
            "views": views,
        }))
        .unwrap_or_default()
    }

    fn tool_list_views(&self) -> String {
        let state = self.state.borrow();
        let views: Vec<Value> = state
            .model
            .views
            .iter()
            .map(|v| {
                json!({
                    "key": v.key,
                    "kind": format!("{:?}", v.kind),
                    "title": v.title,
                    "scope": v.scope,
                })
            })
            .collect();
        serde_json::to_string_pretty(&views).unwrap_or_default()
    }

    fn tool_query(&self, args: &Value) -> String {
        let kind = args.get("kind").and_then(|v| v.as_str());
        let tag = args.get("tag").and_then(|v| v.as_str());
        let name = args.get("name").and_then(|v| v.as_str());

        let state = self.state.borrow();
        let results: Vec<Value> = state
            .model
            .elements
            .values()
            .filter(|el| {
                if let Some(k) = kind {
                    if format!("{:?}", el.kind).to_lowercase() != k.to_lowercase() {
                        return false;
                    }
                }
                if let Some(t) = tag {
                    if !el.tags.iter().any(|et| et.eq_ignore_ascii_case(t)) {
                        return false;
                    }
                }
                if let Some(n) = name {
                    if !el.name.to_lowercase().contains(&n.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .map(element_json)
            .collect();

        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    fn tool_render(&self, args: &Value) -> String {
        let view_key = args.get("view_key").and_then(|v| v.as_str()).unwrap_or("");
        let style = args
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("filled");

        let state = self.state.borrow();
        let view = match state.model.views.iter().find(|v| v.key == view_key) {
            Some(v) => v.clone(),
            None => {
                let keys: Vec<&str> = state.model.views.iter().map(|v| v.key.as_str()).collect();
                return format!(
                    "View '{}' not found. Available: {}",
                    view_key,
                    keys.join(", ")
                );
            }
        };

        let lo = layout::compute_layout(&state.model, &view);
        render::render_svg(&lo, style)
    }

    fn tool_check(&self, args: &Value) -> String {
        let severity_str = args.get("severity").and_then(|v| v.as_str());
        let min = severity_str
            .and_then(check::Severity::from_str)
            .unwrap_or(check::Severity::Warning);

        let state = self.state.borrow();
        let violations = check::check(&state.model, min);
        let results: Vec<Value> = violations
            .iter()
            .map(|v| {
                json!({
                    "severity": v.severity.to_string(),
                    "rule": v.rule,
                    "element": v.element_id,
                    "message": v.message,
                })
            })
            .collect();
        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    fn tool_element_detail(&self, args: &Value) -> String {
        let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let state = self.state.borrow();
        let el = match state.model.elements.get(id) {
            Some(e) => e,
            None => return format!("Element '{}' not found", id),
        };

        let outgoing: Vec<Value> = state
            .model
            .relationships
            .iter()
            .filter(|r| r.frm == id)
            .map(|r| json!({"to": r.to, "label": r.label, "technology": r.technology}))
            .collect();
        let incoming: Vec<Value> = state
            .model
            .relationships
            .iter()
            .filter(|r| r.to == id)
            .map(|r| json!({"from": r.frm, "label": r.label, "technology": r.technology}))
            .collect();
        let children: Vec<Value> = state
            .model
            .elements
            .values()
            .filter(|e| e.parent.as_deref() == Some(id))
            .map(|e| json!({"id": e.id, "kind": format!("{:?}", e.kind), "name": e.name}))
            .collect();

        serde_json::to_string_pretty(&json!({
            "id": el.id, "kind": format!("{:?}", el.kind), "name": el.name,
            "description": el.description, "technology": el.technology,
            "tags": el.tags, "parent": el.parent, "properties": el.properties,
            "children": children,
            "outgoing_relationships": outgoing,
            "incoming_relationships": incoming,
        }))
        .unwrap_or_default()
    }

    fn tool_search(&self, args: &Value) -> String {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let state = self.state.borrow();
        let mut results: Vec<(u32, Value)> = state
            .model
            .elements
            .values()
            .filter_map(|el| {
                let mut score = 0u32;
                if el.name.to_lowercase().contains(&query) {
                    score += 10;
                }
                if el.id.to_lowercase().contains(&query) {
                    score += 5;
                }
                if el
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query)
                {
                    score += 3;
                }
                if el
                    .technology
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query)
                {
                    score += 3;
                }
                if score > 0 {
                    Some((
                        score,
                        json!({"id": el.id, "name": el.name, "kind": format!("{:?}", el.kind), "score": score}),
                    ))
                } else {
                    None
                }
            })
            .collect();
        results.sort_by_key(|r| std::cmp::Reverse(r.0));
        let results: Vec<Value> = results.into_iter().map(|(_, v)| v).collect();
        serde_json::to_string_pretty(&results).unwrap_or_default()
    }

    fn tool_validate(&self, args: &Value) -> String {
        let code = args.get("code").and_then(|v| v.as_str()).unwrap_or("");
        match parser::parse(code) {
            Ok(m) => serde_json::to_string_pretty(&json!({
                "valid": true, "name": m.name,
                "elements": m.elements.len(),
                "relationships": m.relationships.len(),
                "views": m.views.len(),
            }))
            .unwrap_or_default(),
            Err(e) => serde_json::to_string_pretty(&json!({
                "valid": false, "error": e.msg, "line": e.line, "col": e.col,
            }))
            .unwrap_or_default(),
        }
    }
}

fn load_model(source: &Path) -> Result<Model, String> {
    let text =
        std::fs::read_to_string(source).map_err(|e| format!("{}: {}", source.display(), e))?;
    let base_dir = source.parent().unwrap_or(Path::new("."));
    parser::parse_with_preprocess(&text, base_dir).map_err(|e| format!("{}", e))
}

fn element_json(el: &Element) -> Value {
    json!({
        "id": el.id, "kind": format!("{:?}", el.kind), "name": el.name,
        "description": el.description, "technology": el.technology,
        "tags": el.tags, "parent": el.parent,
    })
}

/// Produce a `{ "Container": 12, "Component": 30, ... }` summary. Useful as
/// the very first thing a client sees so it can choose what to drill into.
fn element_kind_counts(model: &Model) -> Value {
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for el in model.elements.values() {
        *counts.entry(format!("{:?}", el.kind)).or_insert(0) += 1;
    }
    serde_json::to_value(counts).unwrap_or_else(|_| json!({}))
}

/// Run the MCP server on stdio.
///
/// When `source` is `Some`, the server pre-loads that `.forge` file at
/// startup. When `None`, the server starts with an empty model; clients
/// bootstrap it by calling `forge_analyze`.
pub fn run(source: Option<PathBuf>) {
    let server = match McpServer::new(source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(response) = server.handle_request(&msg) {
            let out = serde_json::to_string(&response).unwrap_or_default();
            let _ = writeln!(stdout, "{}", out);
            let _ = stdout.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server() -> McpServer {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/payments.forge");
        McpServer::new(Some(source)).expect("should load payments.forge")
    }

    #[test]
    fn initialize() {
        let server = test_server();
        let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}});
        let resp = server.handle_request(&msg).unwrap();
        assert!(resp["result"]["serverInfo"]["name"].as_str() == Some("forge"));
    }

    #[test]
    fn tools_list() {
        let server = test_server();
        let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
        let resp = server.handle_request(&msg).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 10);
        let names: Vec<&str> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap_or(""))
            .collect();
        for expected in [
            "forge_analyze",
            "forge_reload",
            "forge_overview",
            "forge_list_views",
            "forge_query",
            "forge_render",
            "forge_check",
            "forge_element_detail",
            "forge_search",
            "forge_validate",
        ] {
            assert!(names.contains(&expected), "missing tool {}", expected);
        }
    }

    #[test]
    fn query_containers() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_query", "arguments": {"kind": "Container"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let arr: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(arr.len() >= 5);
    }

    #[test]
    fn query_by_tag() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_query", "arguments": {"tag": "database"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let arr: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn render_view() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_render", "arguments": {"view_key": "SystemContext"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("<svg"));
    }

    #[test]
    fn check_violations() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_check", "arguments": {"severity": "info"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let arr: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn element_detail() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_element_detail", "arguments": {"id": "payments.api"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Payment API"));
    }

    #[test]
    fn search() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_search", "arguments": {"query": "payment"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let arr: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(!arr.is_empty());
    }

    #[test]
    fn validate_valid() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_validate", "arguments": {"code": "forge \"T\" { model {} views {} }"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"valid\": true"));
    }

    #[test]
    fn validate_invalid() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_validate", "arguments": {"code": "bad"}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"valid\": false"));
    }

    #[test]
    fn overview_on_loaded_model() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_overview", "arguments": {}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["name"], "Payment Platform");
        assert!(v["counts"]["elements"].as_u64().unwrap() > 10);
        assert!(v["by_kind"]["Container"].as_u64().unwrap() >= 5);
    }

    #[test]
    fn overview_on_empty_server() {
        let server = McpServer::new(None).expect("empty server");
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_overview", "arguments": {}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let v: Value = serde_json::from_str(text).unwrap();
        assert_eq!(v["empty"], true);
    }

    #[test]
    fn list_views_returns_all() {
        let server = test_server();
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_list_views", "arguments": {}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let arr: Vec<Value> = serde_json::from_str(text).unwrap();
        assert!(!arr.is_empty());
        assert!(arr.iter().all(|v| v["key"].is_string()));
    }

    #[test]
    fn analyze_loads_model_from_path() {
        // Analyze the forge crate itself — it has Cargo.toml, a Dockerfile
        // isn't required; the code scanner alone must produce something.
        let server = McpServer::new(None).expect("empty server");
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_analyze", "arguments": {
                "path": crate_dir,
                "scanners": "code",
            }}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let v: Value = serde_json::from_str(text).expect("json");
        assert!(v["elements"].as_u64().unwrap() >= 1, "got {}", v);

        // And the model is now queryable in the same server instance.
        let q = json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "forge_overview", "arguments": {}}
        });
        let resp2 = server.handle_request(&q).unwrap();
        let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
        let v2: Value = serde_json::from_str(text2).unwrap();
        assert!(v2["counts"]["elements"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn reload_without_source_errors() {
        let server = McpServer::new(None).expect("empty server");
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_reload", "arguments": {}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("no source"));
    }

    #[test]
    fn reload_with_source_override() {
        let server = McpServer::new(None).expect("empty server");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/payments.forge");
        let msg = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "forge_reload", "arguments": {"source": source.display().to_string()}}
        });
        let resp = server.handle_request(&msg).unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let v: Value = serde_json::from_str(text).unwrap();
        assert!(v["elements"].as_u64().unwrap() > 5);
    }
}

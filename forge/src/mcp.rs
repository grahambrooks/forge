//! Forge MCP server — JSON-RPC 2.0 over stdio.
//!
//! Implements the Model Context Protocol for AI agent integration.
//! Tools: forge_query, forge_render, forge_check, forge_element_detail,
//! forge_search, forge_validate

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::check;
use crate::layout;
use crate::model::*;
use crate::parser;
use crate::render;

struct McpServer {
    model: Model,
}

impl McpServer {
    fn new(source: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(source).map_err(|e| format!("{}: {}", source.display(), e))?;
        let base_dir = source.parent().unwrap_or(Path::new("."));
        let model = parser::parse_with_preprocess(&text, base_dir).map_err(|e| format!("{}", e))?;
        Ok(Self { model })
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
            "instructions": "Forge architecture model server. Query elements, render diagrams, run checks, and search the model."
        }))
    }

    fn handle_tools_list(&self) -> Result<Value, Value> {
        Ok(json!({
            "tools": [
                {
                    "name": "forge_query",
                    "description": "Query the architecture model. List elements filtered by kind, tag, or name pattern.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "kind": {"type": "string", "description": "Filter by element kind (container, system, person, component, pipeline, stage, branch, deploymentnode)"},
                            "tag": {"type": "string", "description": "Filter by tag (e.g. database, pci)"},
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
                    "description": "Get full details for a specific element by ID.",
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

    fn tool_query(&self, args: &Value) -> String {
        let kind = args.get("kind").and_then(|v| v.as_str());
        let tag = args.get("tag").and_then(|v| v.as_str());
        let name = args.get("name").and_then(|v| v.as_str());

        let results: Vec<Value> = self
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

        let view = match self.model.views.iter().find(|v| v.key == view_key) {
            Some(v) => v,
            None => {
                let keys: Vec<&str> = self.model.views.iter().map(|v| v.key.as_str()).collect();
                return format!(
                    "View '{}' not found. Available: {}",
                    view_key,
                    keys.join(", ")
                );
            }
        };

        let lo = layout::compute_layout(&self.model, view);
        render::render_svg(&lo, style)
    }

    fn tool_check(&self, args: &Value) -> String {
        let severity_str = args.get("severity").and_then(|v| v.as_str());
        let min = severity_str
            .and_then(check::Severity::from_str)
            .unwrap_or(check::Severity::Warning);

        let violations = check::check(&self.model, min);
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
        let el = match self.model.elements.get(id) {
            Some(e) => e,
            None => return format!("Element '{}' not found", id),
        };

        let outgoing: Vec<Value> = self
            .model
            .relationships
            .iter()
            .filter(|r| r.frm == id)
            .map(|r| json!({"to": r.to, "label": r.label, "technology": r.technology}))
            .collect();
        let incoming: Vec<Value> = self
            .model
            .relationships
            .iter()
            .filter(|r| r.to == id)
            .map(|r| json!({"from": r.frm, "label": r.label, "technology": r.technology}))
            .collect();
        let children: Vec<Value> = self
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

        let mut results: Vec<(u32, Value)> = self
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
        results.sort_by(|a, b| b.0.cmp(&a.0));
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

fn element_json(el: &Element) -> Value {
    json!({
        "id": el.id, "kind": format!("{:?}", el.kind), "name": el.name,
        "description": el.description, "technology": el.technology,
        "tags": el.tags, "parent": el.parent,
    })
}

/// Run the MCP server on stdio.
pub fn run(source: PathBuf) {
    let server = match McpServer::new(&source) {
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
        McpServer::new(&source).expect("should load payments.forge")
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
        assert_eq!(tools.len(), 6);
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
}

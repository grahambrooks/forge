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

use crate::model::*;
use crate::parser;

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
}

pub(super) fn load_model(source: &Path) -> Result<Model, String> {
    let text =
        std::fs::read_to_string(source).map_err(|e| format!("{}: {}", source.display(), e))?;
    let base_dir = source.parent().unwrap_or(Path::new("."));
    parser::parse_with_preprocess(&text, base_dir).map_err(|e| format!("{}", e))
}

pub(super) fn element_json(el: &Element) -> Value {
    json!({
        "id": el.id, "kind": format!("{:?}", el.kind), "name": el.name,
        "description": el.description, "technology": el.technology,
        "tags": el.tags, "parent": el.parent,
    })
}

/// Produce a `{ "Container": 12, "Component": 30, ... }` summary. Useful as
/// the very first thing a client sees so it can choose what to drill into.
pub(super) fn element_kind_counts(model: &Model) -> Value {
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

mod tools;

#[cfg(test)]
mod tests;

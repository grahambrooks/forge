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

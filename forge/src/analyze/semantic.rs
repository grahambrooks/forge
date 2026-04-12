//! Semantic source scanner.
//!
//! For each source file:
//!   1. Attribute it to a container via `ContainerIndex` (path-prefix lookup,
//!      not fuzzy slug matching).
//!   2. Run `symgraph::extraction::Extractor::extract_file` to get accurate
//!      symbol and import nodes via tree-sitter.
//!   3. Map symgraph `Import` nodes that resolve to other registered
//!      containers into forge `Relationship`s.
//!   4. Run framework-specific route extractors for endpoint detection.
//!      Tree-sitter doesn't tag decorators as `NodeKind::Route`, so regex
//!      extractors remain the source of truth for HTTP endpoints.
//!   5. Detect database / message-queue usage from literal strings.
//!
//! This replaces the legacy `source.rs` entirely.

use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;

use symgraph::extraction::Extractor;
use symgraph::types::NodeKind;

use crate::model::*;

use super::container_index::ContainerIndex;
use super::provenance::mark_inferred;
use super::AnalyzeConfig;

const SCANNER: &str = "semantic";

pub fn scan(model: &mut Model, index: &ContainerIndex, root: &Path, config: &AnalyzeConfig) {
    let mut extractor = Extractor::new();
    // Build the set of known container ids up-front for import resolution.
    let known_ids: HashSet<String> = model
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Container)
        .map(|e| e.id.clone())
        .collect();

    for entry in WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_source_ext(ext) {
            continue;
        }

        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let container_id = index.attribute(path).map(|s| s.to_string());

        scan_file(
            model,
            &mut extractor,
            path,
            &text,
            container_id.as_deref(),
            &known_ids,
            ext,
        );
    }
}

fn is_source_ext(ext: &str) -> bool {
    matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "java" | "kt" | "go" | "rs" | "py"
    )
}

fn scan_file(
    model: &mut Model,
    extractor: &mut Extractor,
    path: &Path,
    text: &str,
    container_id: Option<&str>,
    known_ids: &HashSet<String>,
    ext: &str,
) {
    // 1) Cross-container import edges from symgraph
    let result = extractor.extract_file(path, text);
    for node in result.nodes.iter().filter(|n| n.kind == NodeKind::Import) {
        if let (Some(from), Some(to)) = (container_id, resolve_import(&node.name, known_ids)) {
            if from != to {
                add_relationship_if_new(model, from, to, "imports");
            }
        }
    }

    // 2) Route detection via regex — tree-sitter grammars don't tag decorators
    //    as NodeKind::Route, so we still scan the text directly.
    for line in text.lines() {
        let trimmed = line.trim();
        let ep = match ext {
            "ts" | "tsx" | "js" | "jsx" | "mjs" => {
                extract_js_route(trimmed).or_else(|| extract_nestjs_route(trimmed))
            }
            "java" | "kt" => extract_spring_route(trimmed).or_else(|| extract_jaxrs_route(trimmed)),
            "go" => extract_go_route(trimmed),
            "rs" => extract_rust_route(trimmed),
            "py" => extract_python_route(trimmed),
            _ => None,
        };
        if let Some(ep) = ep {
            add_endpoint(model, container_id, ep);
        }
    }

    // 3) Infrastructure pattern detection
    scan_infra_patterns(model, text, container_id);
}

/// Best-effort resolution: if an import path (e.g. "../shared-utils" or
/// "@org/api") has a trailing segment matching a known container id, link to it.
fn resolve_import<'a>(import_name: &str, known_ids: &'a HashSet<String>) -> Option<&'a str> {
    let last = import_name
        .trim_matches('"')
        .trim_matches('\'')
        .rsplit('/')
        .next()?;
    known_ids.get(last).map(|s| s.as_str())
}

// ── Route extractors (ported from the legacy source.rs) ─────────────

fn extract_js_route(line: &str) -> Option<ApiEndpoint> {
    for method in &["get", "post", "put", "delete", "patch"] {
        let pattern = format!(".{}(", method);
        if let Some(idx) = line.find(&pattern) {
            let after = &line[idx + pattern.len()..];
            if let Some(path) = extract_quoted_string(after) {
                return Some(ep(method.to_uppercase(), path));
            }
        }
    }
    None
}

fn extract_nestjs_route(line: &str) -> Option<ApiEndpoint> {
    for (decorator, method) in &[
        ("@Get(", "GET"),
        ("@Post(", "POST"),
        ("@Put(", "PUT"),
        ("@Delete(", "DELETE"),
        ("@Patch(", "PATCH"),
    ] {
        if line.contains(decorator) {
            let path = line
                .split(decorator)
                .nth(1)
                .and_then(extract_quoted_string)
                .unwrap_or_else(|| "/".into());
            return Some(ep(method.to_string(), path));
        }
    }
    None
}

fn extract_spring_route(line: &str) -> Option<ApiEndpoint> {
    for (ann, method) in &[
        ("@GetMapping(", "GET"),
        ("@PostMapping(", "POST"),
        ("@PutMapping(", "PUT"),
        ("@DeleteMapping(", "DELETE"),
        ("@PatchMapping(", "PATCH"),
    ] {
        if line.contains(ann) {
            let path = line
                .split(ann)
                .nth(1)
                .and_then(extract_quoted_string)
                .unwrap_or_else(|| "/".into());
            return Some(ep(method.to_string(), path));
        }
    }
    if line.contains("@RequestMapping(") {
        let path = extract_annotation_value(line, "value")
            .or_else(|| {
                line.split("@RequestMapping(")
                    .nth(1)
                    .and_then(extract_quoted_string)
            })
            .unwrap_or_else(|| "/".into());
        let method = extract_annotation_value(line, "method")
            .map(|m| m.replace("RequestMethod.", ""))
            .unwrap_or_else(|| "GET".into());
        return Some(ep(method, path));
    }
    None
}

fn extract_jaxrs_route(line: &str) -> Option<ApiEndpoint> {
    if line.contains("@Path(") {
        let path = line
            .split("@Path(")
            .nth(1)
            .and_then(extract_quoted_string)
            .unwrap_or_else(|| "/".into());
        return Some(ep(String::new(), path));
    }
    for (ann, method) in &[
        ("@GET", "GET"),
        ("@POST", "POST"),
        ("@PUT", "PUT"),
        ("@DELETE", "DELETE"),
    ] {
        if line.contains(ann) && !line.contains("Mapping") {
            return Some(ep(method.to_string(), "/".into()));
        }
    }
    None
}

fn extract_go_route(line: &str) -> Option<ApiEndpoint> {
    for method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
        let pattern = format!(".{}(", method);
        if line.contains(pattern.as_str()) {
            let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
            return Some(ep(method.to_string(), path));
        }
    }
    if line.contains("HandleFunc(") || line.contains(".Handle(") {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        return Some(ep("GET".into(), path));
    }
    None
}

fn extract_rust_route(line: &str) -> Option<ApiEndpoint> {
    for method in &["get", "post", "put", "delete", "patch"] {
        let pattern = format!("#[{}(", method);
        if line.contains(&pattern) {
            let path = line
                .split(&pattern)
                .nth(1)
                .and_then(extract_quoted_string)
                .unwrap_or_else(|| "/".into());
            return Some(ep(method.to_uppercase(), path));
        }
    }
    if line.contains(".route(") {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        let method = if line.contains("get(") {
            "GET"
        } else if line.contains("post(") {
            "POST"
        } else if line.contains("put(") {
            "PUT"
        } else if line.contains("delete(") {
            "DELETE"
        } else {
            "GET"
        };
        return Some(ep(method.into(), path));
    }
    None
}

fn extract_python_route(line: &str) -> Option<ApiEndpoint> {
    if line.contains("@app.route(") || line.contains("@blueprint.route(") {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        let method = if line.contains("POST") {
            "POST"
        } else if line.contains("PUT") {
            "PUT"
        } else if line.contains("DELETE") {
            "DELETE"
        } else {
            "GET"
        };
        return Some(ep(method.into(), path));
    }
    for method in &["get", "post", "put", "delete", "patch"] {
        for pat in &[format!("@app.{}(", method), format!("@router.{}(", method)] {
            if line.contains(pat.as_str()) {
                let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
                return Some(ep(method.to_uppercase(), path));
            }
        }
    }
    if line.contains("path(") && (line.contains("api/") || line.contains("/api")) {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        return Some(ep("GET".into(), path));
    }
    None
}

// ── Infra pattern detection ─────────────────────────────────────────

fn scan_infra_patterns(model: &mut Model, text: &str, container_id: Option<&str>) {
    for line in text.lines() {
        let lower = line.to_lowercase();

        if lower.contains("postgres://")
            || lower.contains("postgresql://")
            || lower.contains("pg_connect")
        {
            ensure_db_element(model, "postgresql", "PostgreSQL", container_id);
        }
        if lower.contains("mysql://") || lower.contains("mysql.createconnection") {
            ensure_db_element(model, "mysql", "MySQL", container_id);
        }
        if lower.contains("mongodb://")
            || lower.contains("mongodb+srv://")
            || lower.contains("mongoclient")
        {
            ensure_db_element(model, "mongodb", "MongoDB", container_id);
        }
        if lower.contains("redis://") || lower.contains("redisclient") || lower.contains("ioredis")
        {
            ensure_db_element(model, "redis", "Redis", container_id);
        }
        if lower.contains("kafka")
            && (lower.contains("producer") || lower.contains("consumer") || lower.contains("topic"))
        {
            ensure_event_infra(model, "kafka", "Apache Kafka", container_id);
        }
        if lower.contains("rabbitmq") || lower.contains("amqp://") {
            ensure_event_infra(model, "rabbitmq", "RabbitMQ", container_id);
        }
        if lower.contains("sqs")
            && (lower.contains("sendmessage")
                || lower.contains("receivemessage")
                || lower.contains("queue"))
        {
            ensure_event_infra(model, "sqs", "AWS SQS", container_id);
        }
        if lower.contains("sns") && (lower.contains("publish") || lower.contains("topic")) {
            ensure_event_infra(model, "sns", "AWS SNS", container_id);
        }
    }
}

fn ensure_db_element(model: &mut Model, id: &str, name: &str, related_to: Option<&str>) {
    let db_id = format!("_inferred_{id}");
    if !model.elements.contains_key(&db_id) {
        let mut el = Element::new(&db_id, ElementKind::Container, name);
        el.technology = Some(name.into());
        el.tags.push("database".into());
        mark_inferred(&mut el, SCANNER, None);
        model.add_element(el);
    }
    if let Some(from) = related_to {
        add_relationship_if_new(model, from, &db_id, "reads/writes");
    }
}

fn ensure_event_infra(model: &mut Model, id: &str, name: &str, related_to: Option<&str>) {
    let infra_id = format!("_inferred_{id}");
    if !model.elements.contains_key(&infra_id) {
        let mut el = Element::new(&infra_id, ElementKind::Container, name);
        el.technology = Some(name.into());
        el.tags.push("messaging".into());
        mark_inferred(&mut el, SCANNER, None);
        model.add_element(el);
    }
    if let Some(from) = related_to {
        add_relationship_if_new(model, from, &infra_id, "publishes/consumes");
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn ep(method: String, path: String) -> ApiEndpoint {
    ApiEndpoint {
        method,
        path,
        description: None,
        request_body: None,
        response: None,
    }
}

fn add_endpoint(model: &mut Model, container_id: Option<&str>, ep: ApiEndpoint) {
    if ep.method.is_empty() && ep.path == "/" {
        return;
    }
    let container = container_id.unwrap_or("_unknown").to_string();
    if let Some(catalog) = model
        .api_catalogs
        .iter_mut()
        .find(|c| c.container == container)
    {
        if !catalog
            .endpoints
            .iter()
            .any(|e| e.method == ep.method && e.path == ep.path)
        {
            catalog.endpoints.push(ep);
        }
    } else {
        model.api_catalogs.push(ApiCatalog {
            container,
            endpoints: vec![ep],
        });
    }
}

fn add_relationship_if_new(model: &mut Model, from: &str, to: &str, label: &str) {
    let exists = model
        .relationships
        .iter()
        .any(|r| r.frm == from && r.to == to && r.label == label);
    if !exists {
        model.add_relationship(Relationship {
            frm: from.into(),
            to: to.into(),
            label: label.into(),
            technology: None,
        });
    }
}

fn extract_quoted_string(s: &str) -> Option<String> {
    for quote in &['"', '\''] {
        if let Some(start) = s.find(*quote) {
            if let Some(end) = s[start + 1..].find(*quote) {
                return Some(s[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn extract_annotation_value(line: &str, key: &str) -> Option<String> {
    let pattern = format!("{key} = ");
    if let Some(idx) = line.find(&pattern) {
        return extract_quoted_string(&line[idx + pattern.len()..]);
    }
    let pattern2 = format!("{key}=");
    if let Some(idx) = line.find(&pattern2) {
        return extract_quoted_string(&line[idx + pattern2.len()..]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_express_and_nest_routes() {
        assert!(extract_js_route("app.get('/users', handler)").is_some());
        let e = extract_nestjs_route("  @Get('/users/:id')").unwrap();
        assert_eq!(e.method, "GET");
        assert_eq!(e.path, "/users/:id");
    }

    #[test]
    fn detect_spring_routes() {
        let e = extract_spring_route("  @GetMapping(\"/api/payments\")").unwrap();
        assert_eq!(e.path, "/api/payments");
    }

    #[test]
    fn detect_go_routes() {
        let e = extract_go_route("  r.GET(\"/users\", getUsers)").unwrap();
        assert_eq!(e.method, "GET");
    }

    #[test]
    fn detect_rust_axum_route() {
        let e = extract_rust_route("  .route(\"/users\", get(list_users))").unwrap();
        assert_eq!(e.method, "GET");
        assert_eq!(e.path, "/users");
    }

    #[test]
    fn detect_fastapi_route() {
        let e = extract_python_route("@app.get(\"/items\")").unwrap();
        assert_eq!(e.method, "GET");
    }

    #[test]
    fn detect_db_pattern() {
        let mut model = Model::default();
        scan_infra_patterns(
            &mut model,
            "let db = postgres://localhost/mydb",
            Some("svc"),
        );
        assert!(model.elements.contains_key("_inferred_postgresql"));
    }

    #[test]
    fn import_resolves_to_known_container() {
        let mut known = HashSet::new();
        known.insert("shared-utils".to_string());
        assert_eq!(
            resolve_import("../shared-utils", &known),
            Some("shared-utils")
        );
        assert_eq!(
            resolve_import("@org/shared-utils", &known),
            Some("shared-utils")
        );
        assert_eq!(resolve_import("unknown-thing", &known), None);
    }
}

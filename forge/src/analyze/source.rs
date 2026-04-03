//! Source-level scanner — scans source files for API routes, import relationships,
//! database connections, and message queue patterns across 6 languages.

use std::path::Path;

use walkdir::WalkDir;

use crate::model::*;

use super::{slugify, AnalyzeConfig};

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "ts" | "tsx" | "js" | "jsx" | "mjs" => scan_js_ts(model, path, root),
            "java" | "kt" => scan_java(model, path, root),
            "go" => scan_go(model, path, root),
            "rs" => scan_rust(model, path, root),
            "py" => scan_python(model, path, root),
            _ => {}
        }
    }
}

/// Get or create a container for a given source directory.
fn container_for_file(model: &Model, path: &Path, root: &Path) -> Option<String> {
    // Walk up from the file to find the nearest known container
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Try matching by directory name against existing containers
    for el in model.elements.values() {
        if el.kind != ElementKind::Container {
            continue;
        }
        let slug = slugify(&el.name);
        for part in &parts {
            if slugify(part) == slug || el.id.contains(part) {
                return Some(el.id.clone());
            }
        }
    }
    None
}

// ─── TypeScript / JavaScript ─────────────────────────────────────

fn scan_js_ts(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let container_id = container_for_file(model, path, root);

    // Express / Koa / Hono routes: app.get('/path', ...), router.post('/path', ...)
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(ep) = extract_js_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
        // NestJS decorators: @Get('/path'), @Post('/path')
        if let Some(ep) = extract_nestjs_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
    }

    // Import detection
    scan_js_imports(model, &text, path, root);

    // Database/queue patterns
    scan_infra_patterns(model, &text, container_id.as_deref());
}

fn extract_js_route(line: &str) -> Option<ApiEndpoint> {
    // app.get('/users', ...) or router.post('/api/payments', ...)
    for method in &["get", "post", "put", "delete", "patch"] {
        let pattern = format!(".{}(", method);
        if let Some(idx) = line.find(&pattern) {
            let after = &line[idx + pattern.len()..];
            if let Some(path) = extract_quoted_string(after) {
                return Some(ApiEndpoint {
                    method: method.to_uppercase(),
                    path,
                    description: None,
                    request_body: None,
                    response: None,
                });
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
            return Some(ApiEndpoint {
                method: method.to_string(),
                path,
                description: None,
                request_body: None,
                response: None,
            });
        }
    }
    None
}

fn scan_js_imports(model: &mut Model, text: &str, path: &Path, root: &Path) {
    let from_id = container_for_file(model, path, root);
    for line in text.lines() {
        let trimmed = line.trim();
        // import ... from '../other-package' or require('../other-package')
        if (trimmed.starts_with("import ") || trimmed.contains("require("))
            && trimmed.contains("../")
        {
            if let Some(target) = extract_import_target(trimmed) {
                if let Some(ref from) = from_id {
                    let to = slugify(&target);
                    if model.elements.contains_key(&to) && to != *from {
                        add_relationship_if_new(model, from, &to, "imports");
                    }
                }
            }
        }
    }
}

// ─── Java / Kotlin ───────────────────────────────────────────────

fn scan_java(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let container_id = container_for_file(model, path, root);

    for line in text.lines() {
        let trimmed = line.trim();
        // Spring: @GetMapping("/path"), @PostMapping("/path"), @RequestMapping(...)
        if let Some(ep) = extract_spring_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
        // JAX-RS: @GET @Path("/path")
        if let Some(ep) = extract_jaxrs_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
    }

    scan_infra_patterns(model, &text, container_id.as_deref());
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
            return Some(ApiEndpoint {
                method: method.to_string(),
                path,
                description: None,
                request_body: None,
                response: None,
            });
        }
    }
    // @RequestMapping(value = "/path", method = RequestMethod.GET)
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
        return Some(ApiEndpoint {
            method,
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    None
}

fn extract_jaxrs_route(line: &str) -> Option<ApiEndpoint> {
    for (ann, method) in &[
        ("@GET", "GET"),
        ("@POST", "POST"),
        ("@PUT", "PUT"),
        ("@DELETE", "DELETE"),
    ] {
        if line.contains(ann) && !line.contains("Mapping") {
            return Some(ApiEndpoint {
                method: method.to_string(),
                path: "/".into(),
                description: None,
                request_body: None,
                response: None,
            });
        }
    }
    if line.contains("@Path(") {
        let path = line
            .split("@Path(")
            .nth(1)
            .and_then(extract_quoted_string)
            .unwrap_or_else(|| "/".into());
        return Some(ApiEndpoint {
            method: String::new(),
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    None
}

// ─── Go ──────────────────────────────────────────────────────────

fn scan_go(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let container_id = container_for_file(model, path, root);

    for line in text.lines() {
        let trimmed = line.trim();
        // gin: r.GET("/path", handler), echo: e.POST("/path", handler)
        // http.HandleFunc("/path", handler)
        if let Some(ep) = extract_go_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
    }

    scan_infra_patterns(model, &text, container_id.as_deref());
}

fn extract_go_route(line: &str) -> Option<ApiEndpoint> {
    for method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
        let patterns = [format!(".{}(", method), format!(".Handle(\"{}\"", method)];
        for pattern in &patterns {
            if line.contains(pattern.as_str()) {
                let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
                return Some(ApiEndpoint {
                    method: method.to_string(),
                    path,
                    description: None,
                    request_body: None,
                    response: None,
                });
            }
        }
    }
    if line.contains("HandleFunc(") || line.contains("Handle(") {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        return Some(ApiEndpoint {
            method: "GET".into(),
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    None
}

// ─── Rust ────────────────────────────────────────────────────────

fn scan_rust(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let container_id = container_for_file(model, path, root);

    for line in text.lines() {
        let trimmed = line.trim();
        // Actix: #[get("/path")], #[post("/path")]
        // Axum: .route("/path", get(handler))
        if let Some(ep) = extract_rust_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
    }

    scan_infra_patterns(model, &text, container_id.as_deref());
}

fn extract_rust_route(line: &str) -> Option<ApiEndpoint> {
    // Actix macros: #[get("/path")], #[post("/path")]
    for method in &["get", "post", "put", "delete", "patch"] {
        let pattern = format!("#[{}(", method);
        if line.contains(&pattern) {
            let path = line
                .split(&pattern)
                .nth(1)
                .and_then(extract_quoted_string)
                .unwrap_or_else(|| "/".into());
            return Some(ApiEndpoint {
                method: method.to_uppercase(),
                path,
                description: None,
                request_body: None,
                response: None,
            });
        }
    }
    // Axum: .route("/path", get(handler))
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
        return Some(ApiEndpoint {
            method: method.into(),
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    None
}

// ─── Python ──────────────────────────────────────────────────────

fn scan_python(model: &mut Model, path: &Path, root: &Path) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let container_id = container_for_file(model, path, root);

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(ep) = extract_python_route(trimmed) {
            add_endpoint(model, container_id.as_deref(), ep);
        }
    }

    scan_infra_patterns(model, &text, container_id.as_deref());
}

fn extract_python_route(line: &str) -> Option<ApiEndpoint> {
    // Flask: @app.route('/path', methods=['GET'])
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
        return Some(ApiEndpoint {
            method: method.into(),
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    // FastAPI: @app.get("/path"), @router.post("/path")
    for method in &["get", "post", "put", "delete", "patch"] {
        let patterns = [format!("@app.{}(", method), format!("@router.{}(", method)];
        for pat in &patterns {
            if line.contains(pat.as_str()) {
                let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
                return Some(ApiEndpoint {
                    method: method.to_uppercase(),
                    path,
                    description: None,
                    request_body: None,
                    response: None,
                });
            }
        }
    }
    // Django: path('api/payments/', views.payments)
    if line.contains("path(") && (line.contains("api/") || line.contains("/api")) {
        let path = extract_quoted_string(line).unwrap_or_else(|| "/".into());
        return Some(ApiEndpoint {
            method: "GET".into(),
            path,
            description: None,
            request_body: None,
            response: None,
        });
    }
    None
}

// ─── Infrastructure pattern detection ────────────────────────────

fn scan_infra_patterns(model: &mut Model, text: &str, container_id: Option<&str>) {
    // Database connections
    for line in text.lines() {
        let lower = line.to_lowercase();

        // PostgreSQL
        if lower.contains("postgres://")
            || lower.contains("postgresql://")
            || lower.contains("pg_connect")
        {
            ensure_db_element(model, "postgresql", "PostgreSQL", container_id);
        }
        // MySQL
        if lower.contains("mysql://") || lower.contains("mysql.createconnection") {
            ensure_db_element(model, "mysql", "MySQL", container_id);
        }
        // MongoDB
        if lower.contains("mongodb://")
            || lower.contains("mongodb+srv://")
            || lower.contains("mongoclient")
        {
            ensure_db_element(model, "mongodb", "MongoDB", container_id);
        }
        // Redis
        if lower.contains("redis://") || lower.contains("redisclient") || lower.contains("ioredis")
        {
            ensure_db_element(model, "redis", "Redis", container_id);
        }

        // Message queues / event streams
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
    let db_id = format!("_inferred_{}", id);
    if !model.elements.contains_key(&db_id) {
        let mut el = Element::new(&db_id, ElementKind::Container, name);
        el.technology = Some(name.into());
        el.tags.push("database".into());
        el.tags.push("inferred".into());
        model.add_element(el);
    }
    if let Some(from) = related_to {
        add_relationship_if_new(model, from, &db_id, "reads/writes");
    }
}

fn ensure_event_infra(model: &mut Model, id: &str, name: &str, related_to: Option<&str>) {
    let infra_id = format!("_inferred_{}", id);
    if !model.elements.contains_key(&infra_id) {
        let mut el = Element::new(&infra_id, ElementKind::Container, name);
        el.technology = Some(name.into());
        el.tags.push("messaging".into());
        el.tags.push("inferred".into());
        model.add_element(el);
    }
    if let Some(from) = related_to {
        add_relationship_if_new(model, from, &infra_id, "publishes/consumes");
    }
}

// ─── Helpers ─────────────────────────────────────────────────────

fn add_endpoint(model: &mut Model, container_id: Option<&str>, ep: ApiEndpoint) {
    if ep.method.is_empty() && ep.path == "/" {
        return;
    }
    let container = container_id.unwrap_or("_unknown").to_string();
    let existing = model
        .api_catalogs
        .iter_mut()
        .find(|c| c.container == container);
    if let Some(catalog) = existing {
        // Avoid duplicate endpoints
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
        .any(|r| r.frm == from && r.to == to);
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
    // Find first quoted string (single or double)
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
    let pattern = format!("{} = ", key);
    if let Some(idx) = line.find(&pattern) {
        let after = &line[idx + pattern.len()..];
        return extract_quoted_string(after);
    }
    let pattern2 = format!("{}=", key);
    if let Some(idx) = line.find(&pattern2) {
        let after = &line[idx + pattern2.len()..];
        return extract_quoted_string(after);
    }
    None
}

fn extract_import_target(line: &str) -> Option<String> {
    // from '../package-name' or require('../package-name')
    if let Some(s) = extract_quoted_string(line) {
        if s.starts_with('.') {
            let parts: Vec<&str> = s.split('/').collect();
            return parts
                .iter()
                .rev()
                .find(|p| !p.starts_with('.') && !p.is_empty())
                .map(|s| s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_express_routes() {
        assert!(extract_js_route("app.get('/users', handler)").is_some());
        assert!(extract_js_route("router.post('/api/payments', ctrl.create)").is_some());
        let ep = extract_js_route("app.get('/health', (req, res) => res.ok())").unwrap();
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/health");
    }

    #[test]
    fn detect_nestjs_routes() {
        let ep = extract_nestjs_route("  @Get('/users/:id')").unwrap();
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/users/:id");
        assert!(extract_nestjs_route("  @Post()").is_some());
    }

    #[test]
    fn detect_spring_routes() {
        let ep = extract_spring_route("  @GetMapping(\"/api/payments\")").unwrap();
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/api/payments");
        assert!(extract_spring_route("  @PostMapping(\"/users\")").is_some());
    }

    #[test]
    fn detect_go_routes() {
        let ep = extract_go_route("  r.GET(\"/users\", getUsers)").unwrap();
        assert_eq!(ep.method, "GET");
        assert!(extract_go_route("  http.HandleFunc(\"/health\", healthCheck)").is_some());
    }

    #[test]
    fn detect_rust_routes() {
        let ep = extract_rust_route("  #[get(\"/api/items\")]").unwrap();
        assert_eq!(ep.method, "GET");
        assert_eq!(ep.path, "/api/items");
        assert!(extract_rust_route("  .route(\"/users\", get(list_users))").is_some());
    }

    #[test]
    fn detect_python_routes() {
        let ep = extract_python_route("@app.get(\"/items\")").unwrap();
        assert_eq!(ep.method, "GET");
        assert!(extract_python_route("@app.route('/users', methods=['POST'])").is_some());
    }

    #[test]
    fn detect_db_patterns() {
        let mut model = Model::default();
        scan_infra_patterns(
            &mut model,
            "let db = postgres://localhost/mydb",
            Some("svc"),
        );
        assert!(model.elements.contains_key("_inferred_postgresql"));
        assert!(model
            .elements
            .get("_inferred_postgresql")
            .unwrap()
            .tags
            .contains(&"database".to_string()));
    }

    #[test]
    fn detect_queue_patterns() {
        let mut model = Model::default();
        scan_infra_patterns(
            &mut model,
            "const producer = new Kafka.Producer()",
            Some("svc"),
        );
        assert!(model.elements.contains_key("_inferred_kafka"));
    }
}

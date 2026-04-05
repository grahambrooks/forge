//! Docker scanner — parses Dockerfiles and docker-compose.yml into container elements.

use std::path::Path;

use walkdir::WalkDir;

use crate::model::*;

use super::{slugify, AnalyzeConfig};

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    scan_dockerfiles(model, root, config);
    scan_compose(model, root);
}

// ── Dockerfile scanning ──────────────────────────────────────────

fn scan_dockerfiles(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "Dockerfile" || name.starts_with("Dockerfile.") {
            if let Ok(text) = std::fs::read_to_string(path) {
                parse_dockerfile(model, path, &text, root);
            }
        }
    }
}

fn parse_dockerfile(model: &mut Model, path: &Path, text: &str, root: &Path) {
    // Derive container name from directory or Dockerfile variant
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let dir_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("service");

    let container_name = if file_name.starts_with("Dockerfile.") {
        // Dockerfile.worker → "worker"
        file_name.strip_prefix("Dockerfile.").unwrap_or(dir_name)
    } else {
        dir_name
    };

    let container_id = slugify(container_name);

    // Skip if already exists (e.g., detected by code scanner)
    if model.elements.contains_key(&container_id) {
        // Enrich existing element with Docker info
        if let Some(el) = model.elements.get_mut(&container_id) {
            let rel_path = path.strip_prefix(root).unwrap_or(path);
            el.properties
                .insert("dockerfile".into(), rel_path.display().to_string());
        }
        return;
    }

    let mut el = Element::new(&container_id, ElementKind::Container, container_name);
    el.tags.push("inferred".into());
    el.tags.push("docker".into());

    let rel_path = path.strip_prefix(root).unwrap_or(path);
    el.properties
        .insert("dockerfile".into(), rel_path.display().to_string());

    // Parse FROM to detect base image / technology
    let mut technology = Vec::new();
    let mut exposes = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(from_arg) = trimmed.strip_prefix("FROM ") {
            let image = from_arg.split_whitespace().next().unwrap_or("");
            // Skip build stages
            if from_arg.contains(" AS ") || from_arg.contains(" as ") {
                continue;
            }
            if let Some(tech) = image_to_technology(image) {
                technology.push(tech.to_string());
            }
        } else if let Some(expose_arg) = trimmed.strip_prefix("EXPOSE ") {
            for port in expose_arg.split_whitespace() {
                exposes.push(port.to_string());
            }
        }
    }

    if !technology.is_empty() {
        el.technology = Some(technology.join(", "));
    }
    if !exposes.is_empty() {
        el.properties.insert("ports".into(), exposes.join(", "));
    }

    let desc = format!("Container built from {}", rel_path.display());
    el.description = Some(desc);

    model.add_element(el);
}

fn image_to_technology(image: &str) -> Option<&str> {
    let base = image.split('/').next_back().unwrap_or(image);
    let name = base.split(':').next().unwrap_or(base);
    match name {
        "node" | "nodejs" => Some("Node.js"),
        "python" => Some("Python"),
        "golang" | "go" => Some("Go"),
        "rust" => Some("Rust"),
        "ruby" => Some("Ruby"),
        "java" | "openjdk" | "eclipse-temurin" => Some("Java"),
        "dotnet" | "aspnet" => Some(".NET"),
        "php" => Some("PHP"),
        "nginx" => Some("Nginx"),
        "postgres" | "postgresql" => Some("PostgreSQL"),
        "mysql" | "mariadb" => Some("MySQL"),
        "redis" => Some("Redis"),
        "mongo" | "mongodb" => Some("MongoDB"),
        "elasticsearch" => Some("Elasticsearch"),
        "rabbitmq" => Some("RabbitMQ"),
        "kafka" => Some("Apache Kafka"),
        _ => None,
    }
}

// ── docker-compose scanning ──────────────────────────────────────

fn scan_compose(model: &mut Model, root: &Path) {
    for name in &[
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ] {
        let path = root.join(name);
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                parse_compose(model, &text);
            }
        }
    }
}

fn parse_compose(model: &mut Model, text: &str) {
    let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let services = match doc.get("services").and_then(|s| s.as_mapping()) {
        Some(s) => s,
        None => return,
    };

    for (svc_key, svc_val) in services {
        let svc_name = match svc_key.as_str() {
            Some(s) => s,
            None => continue,
        };

        let svc_id = slugify(svc_name);

        // Extract image or build context
        let image = svc_val.get("image").and_then(|v| v.as_str());
        let has_build = svc_val.get("build").is_some();

        if model.elements.contains_key(&svc_id) {
            // Enrich existing element
            if let Some(el) = model.elements.get_mut(&svc_id) {
                if let Some(img) = image {
                    el.properties.insert("image".into(), img.into());
                }
            }
        } else {
            let mut el = Element::new(&svc_id, ElementKind::Container, svc_name);
            el.tags.push("inferred".into());
            el.tags.push("docker".into());

            if let Some(img) = image {
                el.properties.insert("image".into(), img.into());
                if let Some(tech) = image_to_technology(img) {
                    el.technology = Some(tech.into());
                }
                // Tag databases
                let db_images = [
                    "postgres",
                    "mysql",
                    "mariadb",
                    "redis",
                    "mongo",
                    "mongodb",
                    "elasticsearch",
                    "memcached",
                    "cassandra",
                ];
                let base = img.split('/').next_back().unwrap_or(img);
                let name_part = base.split(':').next().unwrap_or(base);
                if db_images.iter().any(|d| name_part.contains(d)) {
                    el.tags.push("database".into());
                }
            }

            if has_build {
                el.description = Some("Built from local Dockerfile".to_string());
            } else if let Some(img) = image {
                el.description = Some(format!("Image: {}", img));
            }

            // Extract ports
            if let Some(ports) = svc_val.get("ports").and_then(|p| p.as_sequence()) {
                let port_strs: Vec<String> = ports
                    .iter()
                    .filter_map(|p| p.as_str().map(String::from))
                    .collect();
                if !port_strs.is_empty() {
                    el.properties.insert("ports".into(), port_strs.join(", "));
                }
            }

            model.add_element(el);
        }

        // Parse depends_on as relationships
        if let Some(deps) = svc_val.get("depends_on") {
            let dep_names: Vec<String> = match deps {
                serde_yaml_ng::Value::Sequence(seq) => seq
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                serde_yaml_ng::Value::Mapping(map) => map
                    .keys()
                    .filter_map(|k| k.as_str().map(String::from))
                    .collect(),
                _ => vec![],
            };
            for dep in dep_names {
                let dep_id = slugify(&dep);
                model.add_relationship(Relationship {
                    frm: svc_id.clone(),
                    to: dep_id,
                    label: "depends on".into(),
                    technology: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_dockerfile() {
        let text = r#"
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/myapp /usr/local/bin/
EXPOSE 8080
CMD ["myapp"]
"#;
        let mut model = Model::default();
        parse_dockerfile(
            &mut model,
            Path::new("/project/api/Dockerfile"),
            text,
            Path::new("/project"),
        );

        let api = model.elements.get("api").expect("api container");
        assert_eq!(api.kind, ElementKind::Container);
        assert!(api.tags.contains(&"docker".to_string()));
        // Should not pick up the builder stage's rust image
        assert!(api.technology.is_none() || !api.technology.as_ref().unwrap().contains("Rust"));
        assert_eq!(
            api.properties.get("ports").map(|s| s.as_str()),
            Some("8080")
        );
    }

    #[test]
    fn parse_compose_services() {
        let yaml = r#"
services:
  api:
    build: .
    ports:
      - "8080:8080"
    depends_on:
      - db
      - redis
  db:
    image: postgres:16
    ports:
      - "5432:5432"
  redis:
    image: redis:7-alpine
"#;
        let mut model = Model::default();
        parse_compose(&mut model, yaml);

        assert_eq!(model.elements.len(), 3);

        let db = model.elements.get("db").expect("db");
        assert_eq!(db.technology.as_deref(), Some("PostgreSQL"));
        assert!(db.tags.contains(&"database".to_string()));

        let redis = model.elements.get("redis").expect("redis");
        assert_eq!(redis.technology.as_deref(), Some("Redis"));

        // api depends on db and redis
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "api" && r.to == "db"));
        assert!(model
            .relationships
            .iter()
            .any(|r| r.frm == "api" && r.to == "redis"));
    }

    #[test]
    fn image_technology_detection() {
        assert_eq!(image_to_technology("node:18-alpine"), Some("Node.js"));
        assert_eq!(image_to_technology("postgres:16"), Some("PostgreSQL"));
        assert_eq!(image_to_technology("ghcr.io/org/custom:latest"), None);
    }
}

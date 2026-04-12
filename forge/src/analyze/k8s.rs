//! Kubernetes manifest scanner — parses K8s YAML into deployment nodes and containers.

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
        let ext = path.extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("yml") | Some("yaml")) {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            // K8s manifests have a `kind:` field
            if text.contains("kind:") && (text.contains("apiVersion:") || text.contains("apps/v1"))
            {
                parse_k8s_manifest(model, &text);
            }
        }
    }
}

fn parse_k8s_manifest(model: &mut Model, text: &str) {
    // Handle multi-document YAML (---separator)
    for doc in text.split("\n---") {
        let doc = doc.trim();
        if doc.is_empty() {
            continue;
        }
        let parsed: serde_yaml_ng::Value = match serde_yaml_ng::from_str(doc) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let kind = parsed.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let name = parsed
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let namespace = parsed
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        if name.is_empty() {
            continue;
        }

        match kind {
            "Deployment" | "StatefulSet" | "DaemonSet" => {
                parse_k8s_workload(model, &parsed, name, namespace, kind);
            }
            "Service" => {
                parse_k8s_service(model, &parsed, name, namespace);
            }
            "Ingress" => {
                parse_k8s_ingress(model, &parsed, name, namespace);
            }
            "ConfigMap" => {
                parse_k8s_configmap(model, &parsed, name, namespace);
            }
            _ => {}
        }
    }
}

fn parse_k8s_workload(
    model: &mut Model,
    doc: &serde_yaml_ng::Value,
    name: &str,
    namespace: &str,
    kind: &str,
) {
    let node_id = format!("k8s.{}.{}", namespace, slugify(name));

    // Extract replicas
    let replicas = doc
        .get("spec")
        .and_then(|s| s.get("replicas"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1);

    // Extract the first container spec — we use it for image and env vars.
    let first_container = doc
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_sequence())
        .and_then(|seq| seq.first());

    let image = first_container
        .and_then(|c| c.get("image"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Extract env var names declared on the pod. Supports:
    //   env:
    //     - name: FOO
    //       value: "bar"
    //     - name: BAR
    //       valueFrom:
    //         configMapKeyRef: { name: app-config, key: BAR }
    //     - name: BAZ
    //       valueFrom:
    //         secretKeyRef: { name: app-secrets, key: BAZ }
    //   envFrom:
    //     - configMapRef: { name: shared-config }
    //
    // envFrom references are recorded as the ConfigMap's own key names when
    // the ConfigMap was parsed earlier in the same scan; otherwise they're
    // dropped (we don't know the keys).
    let env_names = extract_k8s_env_names(first_container, model);

    let mut el = Element::new(&node_id, ElementKind::DeploymentNode, name);
    el.technology = Some(format!("{} ({} replicas)", kind, replicas));
    el.tags.push("inferred".into());
    el.tags.push("kubernetes".into());
    el.properties.insert("namespace".into(), namespace.into());
    el.properties
        .insert("replicas".into(), replicas.to_string());
    if !image.is_empty() {
        el.properties.insert("image".into(), image.into());
        el.description = Some(format!("Image: {}", image));
    }
    if !env_names.is_empty() {
        el.properties
            .insert("forge:env_provides".into(), env_names.join(","));
    }

    model.add_element(el);

    // Also attach env_provides to the Container element with the same
    // slugified name (if one exists), so `correlate` can link code-level
    // consumers to the same service the deployment runs. This mirrors how
    // `docker.rs` enriches containers the code scanner already discovered.
    if !env_names.is_empty() {
        let container_id = slugify(name);
        if container_id != node_id {
            if let Some(container) = model.elements.get_mut(&container_id) {
                if container.kind == ElementKind::Container {
                    let mut merged: Vec<String> = container
                        .properties
                        .get("forge:env_provides")
                        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
                        .unwrap_or_default();
                    for n in &env_names {
                        if !merged.iter().any(|m| m == n) {
                            merged.push(n.clone());
                        }
                    }
                    container
                        .properties
                        .insert("forge:env_provides".into(), merged.join(","));
                }
            }
        }
    }
}

/// Collect every env var *name* a pod's first container declares, including
/// those resolved via `valueFrom` and `envFrom`.
fn extract_k8s_env_names(container: Option<&serde_yaml_ng::Value>, model: &Model) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let container = match container {
        Some(c) => c,
        None => return names,
    };

    // Direct `env:` entries.
    if let Some(env) = container.get("env").and_then(|v| v.as_sequence()) {
        for item in env {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                if !name.is_empty() && !names.iter().any(|n| n == name) {
                    names.push(name.to_string());
                }
            }
        }
    }

    // `envFrom:` — pulls every key from a referenced ConfigMap (and, less
    // usefully for correlation, from Secrets where we only know the names).
    // Look up configMapRef.name in model.env_configs (populated earlier by
    // parse_k8s_configmap) and inline each key.
    if let Some(env_from) = container.get("envFrom").and_then(|v| v.as_sequence()) {
        for item in env_from {
            if let Some(cm_name) = item
                .get("configMapRef")
                .and_then(|r| r.get("name"))
                .and_then(|v| v.as_str())
            {
                for cfg in &model.env_configs {
                    // env_configs are keyed "<namespace>/<name>"; match the
                    // trailing segment so we don't have to reconstruct the
                    // namespace here.
                    if cfg.name == cm_name || cfg.name.ends_with(&format!("/{cm_name}")) {
                        for entry in &cfg.entries {
                            if !names.iter().any(|n| n == &entry.key) {
                                names.push(entry.key.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    names
}

fn parse_k8s_service(model: &mut Model, doc: &serde_yaml_ng::Value, name: &str, namespace: &str) {
    // Services create relationships to the pods they select
    let _selector = doc
        .get("spec")
        .and_then(|s| s.get("selector"))
        .and_then(|s| s.as_mapping());

    let svc_type = doc
        .get("spec")
        .and_then(|s| s.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("ClusterIP");

    let ports: Vec<String> = doc
        .get("spec")
        .and_then(|s| s.get("ports"))
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|p| {
                    let port = p.get("port").and_then(|v| v.as_u64())?;
                    Some(format!("{}", port))
                })
                .collect()
        })
        .unwrap_or_default();

    // Try to find a matching workload by name
    let target_id = format!("k8s.{}.{}", namespace, slugify(name));
    if model.elements.contains_key(&target_id) {
        if let Some(el) = model.elements.get_mut(&target_id) {
            el.properties.insert("service_type".into(), svc_type.into());
            if !ports.is_empty() {
                el.properties.insert("ports".into(), ports.join(", "));
            }
        }
    }
}

fn parse_k8s_ingress(model: &mut Model, doc: &serde_yaml_ng::Value, _name: &str, namespace: &str) {
    let rules = doc
        .get("spec")
        .and_then(|s| s.get("rules"))
        .and_then(|r| r.as_sequence());

    if let Some(rules) = rules {
        for rule in rules {
            let host = rule.get("host").and_then(|v| v.as_str()).unwrap_or("*");
            let paths = rule
                .get("http")
                .and_then(|h| h.get("paths"))
                .and_then(|p| p.as_sequence());
            if let Some(paths) = paths {
                for path_rule in paths {
                    let path = path_rule
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("/");
                    let backend = path_rule
                        .get("backend")
                        .and_then(|b| {
                            b.get("service")
                                .and_then(|s| s.get("name"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or("");

                    if !backend.is_empty() {
                        let ep = ApiEndpoint {
                            method: "GET".into(),
                            path: format!("{}{}", host, path),
                            description: Some(format!("Ingress route to {}", backend)),
                            request_body: None,
                            response: None,
                        };
                        let target = format!("k8s.{}.{}", namespace, slugify(backend));
                        // Add as API endpoint on the target
                        let existing = model
                            .api_catalogs
                            .iter_mut()
                            .find(|c| c.container == target);
                        if let Some(catalog) = existing {
                            catalog.endpoints.push(ep);
                        } else {
                            model.api_catalogs.push(ApiCatalog {
                                container: target,
                                endpoints: vec![ep],
                            });
                        }
                    }
                }
            }
        }
    }
}

fn parse_k8s_configmap(model: &mut Model, doc: &serde_yaml_ng::Value, name: &str, namespace: &str) {
    let data = doc.get("data").and_then(|d| d.as_mapping());
    if let Some(data) = data {
        let entries: Vec<ConfigEntry> = data
            .iter()
            .filter_map(|(k, v)| {
                Some(ConfigEntry {
                    key: k.as_str()?.to_string(),
                    value: v.as_str().unwrap_or("***").to_string(),
                })
            })
            .collect();
        if !entries.is_empty() {
            model.env_configs.push(EnvConfig {
                name: format!("{}/{}", namespace, name),
                entries,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_deployment() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: payment-api
  namespace: payments
spec:
  replicas: 3
  template:
    spec:
      containers:
        - name: api
          image: myregistry/payment-api:v1.2.3
"#;
        let mut model = Model::default();
        parse_k8s_manifest(&mut model, yaml);
        let el = model
            .elements
            .get("k8s.payments.payment-api")
            .expect("deployment");
        assert_eq!(el.kind, ElementKind::DeploymentNode);
        assert!(el.technology.as_ref().unwrap().contains("3 replicas"));
        assert!(el.properties.get("image").unwrap().contains("payment-api"));
    }

    #[test]
    fn parse_multi_document() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
  namespace: default
spec:
  replicas: 2
  template:
    spec:
      containers:
        - name: api
          image: api:latest
---
apiVersion: v1
kind: Service
metadata:
  name: api
  namespace: default
spec:
  type: LoadBalancer
  ports:
    - port: 80
"#;
        let mut model = Model::default();
        parse_k8s_manifest(&mut model, yaml);
        let el = model.elements.get("k8s.default.api").expect("deployment");
        assert_eq!(
            el.properties.get("service_type").map(|s| s.as_str()),
            Some("LoadBalancer")
        );
    }

    #[test]
    fn deployment_records_env_provides() {
        let yaml = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api
  namespace: prod
spec:
  replicas: 1
  template:
    spec:
      containers:
        - name: api
          image: api:latest
          env:
            - name: DATABASE_URL
              value: "postgres://db/app"
            - name: JWT_SECRET
              valueFrom:
                secretKeyRef:
                  name: app-secrets
                  key: jwt
"#;
        let mut model = Model::default();
        parse_k8s_manifest(&mut model, yaml);
        let el = model.elements.get("k8s.prod.api").expect("deployment");
        let provides = el
            .properties
            .get("forge:env_provides")
            .expect("env_provides set");
        assert!(provides.contains("DATABASE_URL"));
        assert!(provides.contains("JWT_SECRET"));
    }

    #[test]
    fn envfrom_expands_configmap_keys() {
        // ConfigMap first, then a Deployment that envFroms it.
        let yaml = r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: shared
  namespace: prod
data:
  DATABASE_URL: postgres://db/app
  REDIS_URL: redis://cache:6379
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: worker
  namespace: prod
spec:
  template:
    spec:
      containers:
        - name: worker
          image: worker:latest
          envFrom:
            - configMapRef:
                name: shared
"#;
        let mut model = Model::default();
        parse_k8s_manifest(&mut model, yaml);
        let el = model.elements.get("k8s.prod.worker").expect("deployment");
        let provides = el.properties.get("forge:env_provides").unwrap();
        assert!(provides.contains("DATABASE_URL"));
        assert!(provides.contains("REDIS_URL"));
    }

    #[test]
    fn parse_configmap() {
        let yaml = r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
  namespace: prod
data:
  DATABASE_URL: postgres://db:5432/app
  LOG_LEVEL: info
"#;
        let mut model = Model::default();
        parse_k8s_manifest(&mut model, yaml);
        assert_eq!(model.env_configs.len(), 1);
        assert_eq!(model.env_configs[0].name, "prod/app-config");
    }
}

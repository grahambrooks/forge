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

    // Extract container image
    let image = doc
        .get("spec")
        .and_then(|s| s.get("template"))
        .and_then(|t| t.get("spec"))
        .and_then(|s| s.get("containers"))
        .and_then(|c| c.as_sequence())
        .and_then(|seq| seq.first())
        .and_then(|c| c.get("image"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

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

    model.add_element(el);
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

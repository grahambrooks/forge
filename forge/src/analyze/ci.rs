//! CI scanner — parses GitHub Actions workflow YAML into pipeline elements.

use std::path::Path;

use crate::model::*;

use super::{slugify, AnalyzeConfig};

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    scan_github_actions(model, root, config);
}

fn scan_github_actions(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    let workflows_dir = root.join(".github").join("workflows");
    if !workflows_dir.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(&workflows_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("yml") | Some("yaml")) {
            continue;
        }
        if config.should_exclude(&path) {
            continue;
        }

        if let Ok(text) = std::fs::read_to_string(&path) {
            parse_github_workflow(model, &path, &text);
        }
    }
}

fn parse_github_workflow(model: &mut Model, path: &Path, text: &str) {
    let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("workflow");

    // Workflow name
    let workflow_name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(file_stem);

    let pipeline_id = slugify(workflow_name);

    let mut pipeline = Element::new(&pipeline_id, ElementKind::Pipeline, workflow_name);
    pipeline.tags.push("inferred".into());
    pipeline.tags.push("github-actions".into());

    // Extract trigger info
    // YAML parses bare `on:` as boolean true, so check both string and bool keys
    let on_value = doc.get("on").or_else(|| {
        if let serde_yaml_ng::Value::Mapping(ref map) = doc {
            map.get(serde_yaml_ng::Value::Bool(true))
        } else {
            None
        }
    });
    if let Some(on) = on_value {
        let trigger_desc = describe_triggers(on);
        if !trigger_desc.is_empty() {
            pipeline.description = Some(format!("Triggers: {}", trigger_desc));
        }
    }

    model.add_element(pipeline);

    // Parse jobs as stages
    let jobs = match doc.get("jobs").and_then(|j| j.as_mapping()) {
        Some(j) => j,
        None => return,
    };

    let mut prev_stage_id: Option<String> = None;

    for (job_key, job_val) in jobs {
        let job_id_str = match job_key.as_str() {
            Some(s) => s,
            None => continue,
        };

        let job_name = job_val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(job_id_str);

        let stage_id = format!("{}.{}", pipeline_id, job_id_str);
        let mut stage = Element::new(&stage_id, ElementKind::Stage, job_name);
        stage.parent = Some(pipeline_id.clone());
        stage.tags.push("inferred".into());

        // Extract environment
        if let Some(env) = job_val.get("environment") {
            let env_name = env
                .as_str()
                .or_else(|| env.get("name").and_then(|n| n.as_str()));
            if let Some(e) = env_name {
                stage.properties.insert("environment".into(), e.into());
            }
        }

        // Extract runs-on
        if let Some(runs_on) = job_val.get("runs-on").and_then(|v| v.as_str()) {
            stage.properties.insert("runs-on".into(), runs_on.into());
        }

        model.add_element(stage);

        // Parse "needs" as stage links
        if let Some(needs) = job_val.get("needs") {
            let deps = match needs {
                serde_yaml_ng::Value::String(s) => vec![s.clone()],
                serde_yaml_ng::Value::Sequence(seq) => seq
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => vec![],
            };
            for dep in deps {
                let dep_full = format!("{}.{}", pipeline_id, dep);
                model.stage_links.push(StageLink {
                    frm: dep_full,
                    to: stage_id.clone(),
                });
            }
        } else if let Some(ref prev) = prev_stage_id {
            // Implicit sequential ordering when no explicit needs
            model.stage_links.push(StageLink {
                frm: prev.clone(),
                to: stage_id.clone(),
            });
        }

        // Check for environment protection (implies a gate)
        if job_val.get("environment").is_some() {
            if let Some(env_val) = job_val.get("environment") {
                if env_val.is_mapping() {
                    // environment: { name: ..., url: ... } often implies approval
                    let gate_id = format!("{}.gate", stage_id);
                    let mut gate =
                        Element::new(&gate_id, ElementKind::Gate, "environment-protection");
                    gate.parent = Some(stage_id.clone());
                    gate.tags.push("inferred".into());
                    model.add_element(gate);
                }
            }
        }

        prev_stage_id = Some(stage_id);
    }
}

fn describe_triggers(on: &serde_yaml_ng::Value) -> String {
    match on {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        serde_yaml_ng::Value::Mapping(map) => map
            .keys()
            .filter_map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_workflow() {
        let yaml = r#"
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build
  test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
"#;
        let mut model = Model::default();
        parse_github_workflow(&mut model, Path::new("ci.yml"), yaml);

        let pipeline = model.elements.get("ci").expect("pipeline");
        assert_eq!(pipeline.kind, ElementKind::Pipeline);
        assert_eq!(pipeline.name, "CI");
        assert!(pipeline.description.as_ref().unwrap().contains("push"));

        let build = model.elements.get("ci.build").expect("build stage");
        assert_eq!(build.kind, ElementKind::Stage);
        assert_eq!(build.parent.as_deref(), Some("ci"));

        let test = model.elements.get("ci.test").expect("test stage");
        assert_eq!(test.name, "test");

        // needs: build creates a stage link
        assert!(model
            .stage_links
            .iter()
            .any(|l| l.frm == "ci.build" && l.to == "ci.test"));
    }

    #[test]
    fn parse_workflow_with_named_jobs() {
        let yaml = r#"
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy-staging:
    name: Deploy to Staging
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - run: deploy.sh
  deploy-prod:
    name: Deploy to Production
    needs: deploy-staging
    runs-on: ubuntu-latest
    environment:
      name: production
      url: https://example.com
    steps:
      - run: deploy.sh
"#;
        let mut model = Model::default();
        parse_github_workflow(&mut model, Path::new("deploy.yml"), yaml);

        let staging = model
            .elements
            .get("deploy.deploy-staging")
            .expect("staging");
        assert_eq!(staging.name, "Deploy to Staging");
        assert_eq!(
            staging.properties.get("environment").map(|s| s.as_str()),
            Some("staging")
        );

        let prod = model.elements.get("deploy.deploy-prod").expect("prod");
        assert_eq!(
            prod.properties.get("environment").map(|s| s.as_str()),
            Some("production")
        );

        // Production with mapping environment gets a gate
        assert!(model.elements.get("deploy.deploy-prod.gate").is_some());
    }

    #[test]
    fn slugify_names() {
        assert_eq!(slugify("CI/CD Pipeline"), "ci-cd-pipeline");
        assert_eq!(slugify("Build & Test"), "build-test");
    }
}

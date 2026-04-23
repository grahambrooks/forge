//! Infrastructure scanner — parses CloudFormation and Terraform into deployment elements.

use std::path::Path;

use walkdir::WalkDir;

use crate::model::*;

use super::provenance::mark_inferred;
use super::{slugify, AnalyzeConfig};

const SCANNER: &str = "infra";

pub fn scan(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    scan_cloudformation(model, root, config);
    scan_terraform(model, root, config);
    scan_openapi(model, root, config);
}

// ─── CloudFormation ──────────────────────────────────────────────

fn scan_cloudformation(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let _name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str());
        // CFT files are usually named template.yaml, cfn-*.yaml, or have AWSTemplateFormatVersion
        let is_yaml = matches!(ext, Some("yml") | Some("yaml"));
        let is_json = matches!(ext, Some("json"));
        if !is_yaml && !is_json {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            if text.contains("AWSTemplateFormatVersion") || text.contains("AWS::") {
                parse_cft(model, &text);
            }
        }
    }
}

fn parse_cft(model: &mut Model, text: &str) {
    let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            // Try as JSON
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
                let yaml_str = serde_yaml_ng::to_string(&v).unwrap_or_default();
                match serde_yaml_ng::from_str(&yaml_str) {
                    Ok(v) => v,
                    Err(_) => return,
                }
            } else {
                return;
            }
        }
    };

    let resources = match doc.get("Resources").and_then(|r| r.as_mapping()) {
        Some(r) => r,
        None => return,
    };

    for (key, val) in resources {
        let resource_name = key.as_str().unwrap_or("");
        let resource_type = val.get("Type").and_then(|t| t.as_str()).unwrap_or("");

        let (kind, technology, tags) = classify_aws_resource(resource_type);
        if kind.is_none() {
            continue;
        }

        let el_id = format!("cft.{}", slugify(resource_name));
        let mut el = Element::new(&el_id, kind.unwrap(), resource_name);
        el.technology = Some(technology);
        mark_inferred(&mut el, SCANNER, None);
        el.tags.push("aws".into());
        for tag in tags {
            el.tags.push(tag.into());
        }
        el.description = Some(format!("CloudFormation: {}", resource_type));

        model.add_element(el);
    }
}

fn classify_aws_resource(resource_type: &str) -> (Option<ElementKind>, String, Vec<&str>) {
    match resource_type {
        "AWS::Lambda::Function" => (Some(ElementKind::Container), "AWS Lambda".into(), vec![]),
        "AWS::ECS::Service" | "AWS::ECS::TaskDefinition" => {
            (Some(ElementKind::Container), "AWS ECS".into(), vec![])
        }
        "AWS::EC2::Instance" => (Some(ElementKind::DeploymentNode), "AWS EC2".into(), vec![]),
        "AWS::RDS::DBInstance" | "AWS::RDS::DBCluster" => (
            Some(ElementKind::Container),
            "AWS RDS".into(),
            vec!["database"],
        ),
        "AWS::DynamoDB::Table" => (
            Some(ElementKind::Container),
            "AWS DynamoDB".into(),
            vec!["database"],
        ),
        "AWS::ElastiCache::CacheCluster" | "AWS::ElastiCache::ReplicationGroup" => (
            Some(ElementKind::Container),
            "AWS ElastiCache".into(),
            vec!["database"],
        ),
        "AWS::S3::Bucket" => (Some(ElementKind::Container), "AWS S3".into(), vec![]),
        "AWS::SQS::Queue" => (
            Some(ElementKind::Container),
            "AWS SQS".into(),
            vec!["messaging"],
        ),
        "AWS::SNS::Topic" => (
            Some(ElementKind::Container),
            "AWS SNS".into(),
            vec!["messaging"],
        ),
        "AWS::ApiGateway::RestApi" | "AWS::ApiGatewayV2::Api" => (
            Some(ElementKind::Container),
            "AWS API Gateway".into(),
            vec![],
        ),
        "AWS::EKS::Cluster" => (
            Some(ElementKind::DeploymentNode),
            "AWS EKS".into(),
            vec!["kubernetes"],
        ),
        "AWS::CloudFront::Distribution" => (
            Some(ElementKind::Container),
            "AWS CloudFront".into(),
            vec![],
        ),
        "AWS::Kinesis::Stream" => (
            Some(ElementKind::Container),
            "AWS Kinesis".into(),
            vec!["messaging"],
        ),
        _ => (None, String::new(), vec![]),
    }
}

// ─── Terraform ───────────────────────────────────────────────────

fn scan_terraform(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("tf") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            parse_terraform(model, &text);
        }
    }
}

fn parse_terraform(model: &mut Model, text: &str) {
    // Parse resource "type" "name" { ... } blocks
    let mut pos = 0;
    while pos < text.len() {
        if let Some(idx) = text[pos..].find("resource ") {
            let start = pos + idx + 9; // after "resource "
                                       // Extract "type" "name"
            let rest = &text[start..];
            let parts: Vec<&str> = rest.splitn(3, '"').collect();
            if parts.len() >= 4 {
                // parts: ["", type, " ", ...]
                // Actually: split on " gives: empty, type, space, name, rest
            }
            // Simpler: find the two quoted strings
            if let (Some(rt), remaining) = extract_tf_string(rest) {
                if let (Some(rn), _) = extract_tf_string(remaining) {
                    let (kind, technology, tags) = classify_tf_resource(&rt);
                    if let Some(k) = kind {
                        let el_id = format!("tf.{}", slugify(&rn));
                        let mut el = Element::new(&el_id, k, &rn);
                        el.technology = Some(technology);
                        mark_inferred(&mut el, SCANNER, None);
                        el.tags.push("terraform".into());
                        for tag in tags {
                            el.tags.push(tag.into());
                        }
                        el.description = Some(format!("Terraform: {}", rt));
                        model.add_element(el);
                    }
                }
            }
            pos = start + 1;
        } else {
            break;
        }
    }
}

fn extract_tf_string(s: &str) -> (Option<String>, &str) {
    if let Some(start) = s.find('"') {
        let rest = &s[start + 1..];
        if let Some(end) = rest.find('"') {
            return (Some(rest[..end].to_string()), &rest[end + 1..]);
        }
    }
    (None, s)
}

fn classify_tf_resource(resource_type: &str) -> (Option<ElementKind>, String, Vec<&str>) {
    // AWS resources
    if resource_type.starts_with("aws_") {
        return match resource_type {
            "aws_lambda_function" => (Some(ElementKind::Container), "AWS Lambda".into(), vec![]),
            "aws_ecs_service" | "aws_ecs_task_definition" => {
                (Some(ElementKind::Container), "AWS ECS".into(), vec![])
            }
            "aws_instance" => (Some(ElementKind::DeploymentNode), "AWS EC2".into(), vec![]),
            "aws_db_instance" | "aws_rds_cluster" => (
                Some(ElementKind::Container),
                "AWS RDS".into(),
                vec!["database"],
            ),
            "aws_dynamodb_table" => (
                Some(ElementKind::Container),
                "AWS DynamoDB".into(),
                vec!["database"],
            ),
            "aws_elasticache_cluster" | "aws_elasticache_replication_group" => (
                Some(ElementKind::Container),
                "AWS ElastiCache".into(),
                vec!["database"],
            ),
            "aws_s3_bucket" => (Some(ElementKind::Container), "AWS S3".into(), vec![]),
            "aws_sqs_queue" => (
                Some(ElementKind::Container),
                "AWS SQS".into(),
                vec!["messaging"],
            ),
            "aws_sns_topic" => (
                Some(ElementKind::Container),
                "AWS SNS".into(),
                vec!["messaging"],
            ),
            "aws_api_gateway_rest_api" | "aws_apigatewayv2_api" => (
                Some(ElementKind::Container),
                "AWS API Gateway".into(),
                vec![],
            ),
            "aws_eks_cluster" => (
                Some(ElementKind::DeploymentNode),
                "AWS EKS".into(),
                vec!["kubernetes"],
            ),
            "aws_cloudfront_distribution" => (
                Some(ElementKind::Container),
                "AWS CloudFront".into(),
                vec![],
            ),
            _ => (None, String::new(), vec![]),
        };
    }
    // GCP resources
    if resource_type.starts_with("google_") {
        return match resource_type {
            "google_cloud_run_service" => {
                (Some(ElementKind::Container), "Cloud Run".into(), vec![])
            }
            "google_sql_database_instance" => (
                Some(ElementKind::Container),
                "Cloud SQL".into(),
                vec!["database"],
            ),
            "google_container_cluster" => (
                Some(ElementKind::DeploymentNode),
                "GKE".into(),
                vec!["kubernetes"],
            ),
            "google_pubsub_topic" => (
                Some(ElementKind::Container),
                "Pub/Sub".into(),
                vec!["messaging"],
            ),
            "google_storage_bucket" => {
                (Some(ElementKind::Container), "Cloud Storage".into(), vec![])
            }
            _ => (None, String::new(), vec![]),
        };
    }
    // Azure resources
    if resource_type.starts_with("azurerm_") {
        return match resource_type {
            "azurerm_function_app" => (
                Some(ElementKind::Container),
                "Azure Functions".into(),
                vec![],
            ),
            "azurerm_container_group" => (
                Some(ElementKind::Container),
                "Azure Container Instances".into(),
                vec![],
            ),
            "azurerm_kubernetes_cluster" => (
                Some(ElementKind::DeploymentNode),
                "AKS".into(),
                vec!["kubernetes"],
            ),
            "azurerm_mssql_server" | "azurerm_postgresql_server" => (
                Some(ElementKind::Container),
                "Azure SQL".into(),
                vec!["database"],
            ),
            "azurerm_servicebus_queue" => (
                Some(ElementKind::Container),
                "Azure Service Bus".into(),
                vec!["messaging"],
            ),
            "azurerm_storage_account" => {
                (Some(ElementKind::Container), "Azure Storage".into(), vec![])
            }
            _ => (None, String::new(), vec![]),
        };
    }
    (None, String::new(), vec![])
}

// ─── OpenAPI / Swagger ───────────────────────────────────────────

fn scan_openapi(model: &mut Model, root: &Path, config: &AnalyzeConfig) {
    for entry in WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if config.should_exclude(path) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("openapi") || name.starts_with("swagger") {
            if let Ok(text) = std::fs::read_to_string(path) {
                if text.contains("openapi") || text.contains("swagger") {
                    parse_openapi(model, &text, path, root);
                }
            }
        }
    }
}

fn parse_openapi(model: &mut Model, text: &str, _path: &Path, _root: &Path) {
    let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            // Try JSON
            match serde_json::from_str::<serde_json::Value>(text) {
                Ok(v) => {
                    let yaml = serde_yaml_ng::to_string(&v).unwrap_or_default();
                    match serde_yaml_ng::from_str(&yaml) {
                        Ok(v) => v,
                        Err(_) => return,
                    }
                }
                Err(_) => return,
            }
        }
    };

    let api_title = doc
        .get("info")
        .and_then(|i| i.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or("API");

    let container_id = format!("openapi.{}", slugify(api_title));
    if !model.elements.contains_key(&container_id) {
        let mut el = Element::new(&container_id, ElementKind::Container, api_title);
        mark_inferred(&mut el, SCANNER, None);
        el.tags.push("openapi".into());
        let desc = doc
            .get("info")
            .and_then(|i| i.get("description"))
            .and_then(|d| d.as_str());
        if let Some(d) = desc {
            el.description = Some(d.to_string());
        }
        model.add_element(el);
    }

    // Parse paths
    let paths = doc.get("paths").and_then(|p| p.as_mapping());
    if let Some(paths) = paths {
        let mut endpoints = Vec::new();
        for (path_key, methods) in paths {
            let path_str = path_key.as_str().unwrap_or("/");
            if let Some(methods) = methods.as_mapping() {
                for (method_key, op) in methods {
                    let method = method_key.as_str().unwrap_or("get").to_uppercase();
                    if matches!(method.as_str(), "GET" | "POST" | "PUT" | "DELETE" | "PATCH") {
                        let description = op
                            .get("summary")
                            .and_then(|s| s.as_str())
                            .or_else(|| op.get("description").and_then(|s| s.as_str()));
                        endpoints.push(ApiEndpoint {
                            method,
                            path: path_str.to_string(),
                            description: description.map(String::from),
                            request_body: None,
                            response: None,
                        });
                    }
                }
            }
        }
        if !endpoints.is_empty() {
            model.api_catalogs.push(ApiCatalog {
                container: container_id,
                endpoints,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cft_lambda() {
        let yaml = r#"
AWSTemplateFormatVersion: '2010-09-09'
Resources:
  PaymentFunction:
    Type: AWS::Lambda::Function
    Properties:
      FunctionName: payment-handler
      Runtime: nodejs18.x
  OrdersTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: orders
  PaymentQueue:
    Type: AWS::SQS::Queue
    Properties:
      QueueName: payments
"#;
        let mut model = Model::default();
        parse_cft(&mut model, yaml);
        assert!(model.elements.contains_key("cft.paymentfunction"));
        assert!(model.elements.contains_key("cft.orderstable"));
        assert!(model
            .elements
            .get("cft.orderstable")
            .unwrap()
            .tags
            .contains(&"database".to_string()));
        assert!(model
            .elements
            .get("cft.paymentqueue")
            .unwrap()
            .tags
            .contains(&"messaging".to_string()));
    }

    #[test]
    fn parse_terraform_aws() {
        let tf = r#"
resource "aws_lambda_function" "api" {
  function_name = "payment-api"
  runtime       = "nodejs18.x"
}

resource "aws_dynamodb_table" "orders" {
  name = "orders"
}

resource "aws_sqs_queue" "notifications" {
  name = "notifications"
}
"#;
        let mut model = Model::default();
        parse_terraform(&mut model, tf);
        assert!(model.elements.contains_key("tf.api"));
        assert!(model.elements.contains_key("tf.orders"));
        assert!(model
            .elements
            .get("tf.orders")
            .unwrap()
            .tags
            .contains(&"database".to_string()));
    }

    #[test]
    fn parse_terraform_gcp() {
        let tf = r#"
resource "google_cloud_run_service" "api" {
  name = "my-api"
}
resource "google_sql_database_instance" "db" {
  name = "my-db"
}
"#;
        let mut model = Model::default();
        parse_terraform(&mut model, tf);
        assert!(model.elements.contains_key("tf.api"));
        assert!(model
            .elements
            .get("tf.api")
            .unwrap()
            .technology
            .as_ref()
            .unwrap()
            .contains("Cloud Run"));
        assert!(model
            .elements
            .get("tf.db")
            .unwrap()
            .tags
            .contains(&"database".to_string()));
    }

    #[test]
    fn parse_openapi_spec() {
        let yaml = r#"
openapi: 3.0.0
info:
  title: Payment API
  description: Handles payments
paths:
  /payments:
    get:
      summary: List payments
    post:
      summary: Create payment
  /payments/{id}:
    get:
      summary: Get payment by ID
"#;
        let mut model = Model::default();
        parse_openapi(&mut model, yaml, Path::new("openapi.yaml"), Path::new("."));
        assert!(model.elements.contains_key("openapi.payment-api"));
        assert_eq!(model.api_catalogs.len(), 1);
        assert_eq!(model.api_catalogs[0].endpoints.len(), 3);
    }
}

use super::*;

fn payments_model() -> Model {
    let text = include_str!("../../examples/payments.forge");
    parse(text).expect("payments.forge should parse")
}

#[test]
fn parse_model_name() {
    let m = payments_model();
    assert_eq!(m.name, "Payment Platform");
}

#[test]
fn parse_element_counts() {
    let m = payments_model();
    assert_eq!(m.elements.len(), 30); // 16 structural/process + 4 components + 8 deployment + 2 branches
    assert_eq!(m.relationships.len(), 11); // 5 model + 4 component + 2 branch flows
    assert_eq!(m.views.len(), 13); // +1 component, +1 animated, +1 apiCatalog, +1 eventFlow
}

#[test]
fn parse_person() {
    let m = payments_model();
    let cust = m.elements.get("customer").expect("customer element");
    assert_eq!(cust.kind, ElementKind::Person);
    assert_eq!(cust.name, "Customer");
    assert_eq!(
        cust.description.as_deref(),
        Some("End user making payments")
    );
    assert!(cust.parent.is_none());
}

#[test]
fn parse_system_with_children() {
    let m = payments_model();
    let sys = m.elements.get("payments").expect("payments system");
    assert_eq!(sys.kind, ElementKind::System);
    assert_eq!(sys.name, "Payment Service");
    assert!(sys.tags.contains(&"core".to_string()));
    assert!(sys.tags.contains(&"pci".to_string()));
    assert!(!sys.children.is_empty());
}

#[test]
fn parse_container_technology() {
    let m = payments_model();
    let api = m.elements.get("payments.api").expect("api container");
    assert_eq!(api.kind, ElementKind::Container);
    assert_eq!(api.technology.as_deref(), Some("Rust / Actix"));
    assert_eq!(api.parent.as_deref(), Some("payments"));
}

#[test]
fn parse_database_tags() {
    let m = payments_model();
    let db = m.elements.get("payments.db").expect("db container");
    assert!(db.tags.contains(&"database".to_string()));
    assert_eq!(db.technology.as_deref(), Some("PostgreSQL 16"));
}

#[test]
fn parse_data_class_keyword() {
    let src = r#"
forge "t" {
  model {
    sys = system "Sys" {
      db = container "Ledger" {
        technology "PostgreSQL"
        tags "database"
        data-class "pii" "financial"
      }
    }
  }
}
"#;
    let m = parse(src).unwrap();
    let db = m.elements.get("sys.db").expect("db container");
    assert_eq!(
        db.data_classes,
        vec!["pii".to_string(), "financial".to_string()]
    );
    // data-class doesn't pollute tags
    assert!(!db.tags.contains(&"pii".to_string()));
}

#[test]
fn parse_dynamic_view_with_ordered_relationships() {
    let src = r#"
forge "Login" {
  model {
    user = person "User"
    app = system "Web App" {
      web = container "Web UI"
      api = container "API"
      db = container "DB"
    }
    user -> app.web "uses"
  }

  views {
    dynamic-view app "LoginFlow" {
      title "User Login Flow"
      1. user -> app.web "submits credentials" "HTTPS"
      2. app.web -> app.api "POST /login"
      3. app.api -> app.db "SELECT user"
    }
  }
}
"#;
    let m = parse(src).unwrap();
    assert_eq!(m.views.len(), 1);
    let view = &m.views[0];
    assert_eq!(view.kind, ViewKind::Dynamic);
    assert_eq!(view.key, "LoginFlow");
    assert_eq!(view.title.as_deref(), Some("User Login Flow"));

    // Ordered relationships landed with step numbers
    let ordered: Vec<(&str, &str, u32)> = m
        .relationships
        .iter()
        .filter(|r| r.order.is_some())
        .map(|r| (r.frm.as_str(), r.to.as_str(), r.order.unwrap()))
        .collect();
    assert_eq!(ordered.len(), 3);
    assert!(ordered.contains(&("user", "app.web", 1)));
    assert!(ordered.contains(&("app.web", "app.api", 2)));
    assert!(ordered.contains(&("app.api", "app.db", 3)));

    // The unordered relationship from the model block is preserved
    assert!(m
        .relationships
        .iter()
        .any(|r| r.frm == "user" && r.to == "app.web" && r.order.is_none()));
}

#[test]
fn parse_composite_view() {
    let src = r#"
forge "Dash" {
  model {
    sys = system "S" {
      api = container "API"
    }
  }
  views {
    system-context-view sys "Context" {
      include *
    }
    container-view sys "Containers" {
      include *
    }
    composite-view "Dashboard" {
      title "Exec Dashboard"
      grid 2 1
      cell "Context"
      cell "Containers"
    }
  }
}
"#;
    let m = parse(src).unwrap();
    let comp_view = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Composite)
        .expect("composite view");
    let comp = comp_view.composite.as_ref().expect("composite payload");
    assert_eq!(comp.cols, 2);
    assert_eq!(comp.rows, 1);
    assert_eq!(
        comp.cells,
        vec!["Context".to_string(), "Containers".to_string()]
    );
    assert_eq!(comp_view.title.as_deref(), Some("Exec Dashboard"));
}

#[test]
fn parse_data_class_empty_by_default() {
    let m = payments_model();
    let api = m.elements.get("payments.api").unwrap();
    assert!(api.data_classes.is_empty());
}

#[test]
fn parse_relationships() {
    let m = payments_model();
    let api_to_proc = m
        .relationships
        .iter()
        .find(|r| r.frm == "payments.api" && r.to == "payments.processor");
    assert!(api_to_proc.is_some());
    let rel = api_to_proc.unwrap();
    assert_eq!(rel.label, "delegates to");
    assert_eq!(rel.technology.as_deref(), Some("gRPC"));
}

#[test]
fn parse_cross_scope_relationship() {
    let m = payments_model();
    let cust_to_api = m
        .relationships
        .iter()
        .find(|r| r.frm == "customer" && r.to == "payments.api");
    assert!(cust_to_api.is_some());
    assert_eq!(cust_to_api.unwrap().label, "makes payments");
}

#[test]
fn parse_pipeline() {
    let m = payments_model();
    let pipeline = m.elements.get("payments-ci").expect("pipeline element");
    assert_eq!(pipeline.kind, ElementKind::Pipeline);
}

#[test]
fn parse_stages() {
    let m = payments_model();
    let stages: Vec<_> = m
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Stage)
        .collect();
    assert_eq!(stages.len(), 4);
    let build = m.elements.get("payments-ci.build").expect("build stage");
    assert_eq!(build.name, "Build & Test");
    assert_eq!(build.parent.as_deref(), Some("payments-ci"));
}

#[test]
fn parse_stage_links() {
    let m = payments_model();
    assert_eq!(m.stage_links.len(), 3);
    assert!(m
        .stage_links
        .iter()
        .any(|l| l.frm == "payments-ci.build" && l.to == "payments-ci.security"));
}

#[test]
fn parse_gates() {
    let m = payments_model();
    let gates: Vec<_> = m
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Gate)
        .collect();
    assert_eq!(gates.len(), 3);
    let manual = m.elements.get("payments-ci.prod.gate").expect("prod gate");
    assert_eq!(manual.name, "manual-approval");
    assert_eq!(
        manual.properties.get("approvers").map(|s| s.as_str()),
        Some("platform-team")
    );
}

#[test]
fn parse_views() {
    let m = payments_model();
    let sc = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::SystemContext)
        .unwrap();
    assert_eq!(sc.key, "SystemContext");
    assert_eq!(sc.auto_layout, AutoLayout::LeftRight);
    assert!(sc.include_all);

    let cont = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Container)
        .unwrap();
    assert_eq!(cont.key, "Containers");
    assert_eq!(cont.auto_layout, AutoLayout::TopBottom);

    let pipe = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::PipelineView)
        .unwrap();
    assert_eq!(pipe.key, "Pipeline");
    assert_eq!(pipe.scope.as_deref(), Some("payments-ci"));
}

#[test]
fn parse_view_titles() {
    let m = payments_model();
    let sc = m.views.iter().find(|v| v.key == "SystemContext").unwrap();
    assert_eq!(
        sc.title.as_deref(),
        Some("Payment Platform — System Context")
    );
}

#[test]
fn parse_minimal() {
    let m = parse(r#"forge "Tiny" { model {} views {} }"#).unwrap();
    assert_eq!(m.name, "Tiny");
    assert!(m.elements.is_empty());
    assert!(m.views.is_empty());
}

#[test]
fn parse_error_missing_forge() {
    let err = parse(r#"notforge "X" {}"#);
    assert!(err.is_err());
    assert!(err.unwrap_err().msg.contains("expected 'forge'"));
}

#[test]
fn parse_error_unterminated_string() {
    let err = parse(r#"forge "oops"#);
    assert!(err.is_err());
}

#[test]
fn parse_comments_ignored() {
    let m = parse(
        r#"
        forge "Test" {
            // this is a comment
            model {
                // another comment
                a = person "Alice" {}
            }
            views {}
        }
    "#,
    )
    .unwrap();
    assert_eq!(m.elements.len(), 1);
}

#[test]
fn parse_repository() {
    let m = payments_model();
    let repo = m.elements.get("repo").expect("repository element");
    assert_eq!(repo.kind, ElementKind::Repository);
    assert_eq!(repo.name, "payments-api");
    assert!(repo.properties.contains_key("url"));
}

#[test]
fn parse_docs() {
    let m = payments_model();
    assert_eq!(m.docs.len(), 5);
    assert_eq!(m.docs[0].title, "Overview");
    assert_eq!(m.docs[0].path, "docs/overview.md");
    assert_eq!(m.docs[1].title, "Architecture Decisions");
    assert_eq!(m.docs[3].title, "Security");
    assert_eq!(m.docs[4].title, "ADR-006: Async Notifications and Caching");
}

#[test]
fn parse_docs_empty() {
    let m = parse(r#"forge "X" { docs {} model {} views {} }"#).unwrap();
    assert!(m.docs.is_empty());
}

#[test]
fn parse_deployment_nodes() {
    let m = payments_model();
    let deploy_nodes: Vec<_> = m
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::DeploymentNode)
        .collect();
    assert_eq!(deploy_nodes.len(), 8); // AWS, us-east-1, EKS, API Pods, Processor Pods, Notification Pods, RDS, ElastiCache

    // Check nesting
    let eks = deploy_nodes
        .iter()
        .find(|e| e.name == "EKS Cluster")
        .unwrap();
    assert!(eks.technology.as_deref() == Some("Kubernetes 1.29"));

    // Check container instances
    let rds = deploy_nodes.iter().find(|e| e.name == "RDS").unwrap();
    assert!(rds
        .properties
        .get("container_instances")
        .unwrap()
        .contains("payments.db"));
}

#[test]
fn parse_deployment_view() {
    let m = payments_model();
    let dv = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Deployment)
        .unwrap();
    assert_eq!(dv.key, "Deployment");
    assert_eq!(dv.scope.as_deref(), Some("production"));
}

#[test]
fn parse_tech_stack() {
    let m = payments_model();
    assert_eq!(m.tech_stack.len(), 5);
    assert_eq!(m.tech_stack[0].name, "Languages & Frameworks");
    assert_eq!(m.tech_stack[0].entries.len(), 4);

    let rust = &m.tech_stack[0].entries[0];
    assert_eq!(rust.name, "Rust");
    assert_eq!(rust.version.as_deref(), Some("1.75"));
    assert_eq!(rust.purpose.as_deref(), Some("Payment API and Processor"));

    // Data stores
    assert_eq!(m.tech_stack[1].name, "Data Stores");
    assert_eq!(m.tech_stack[1].entries.len(), 2);
}

#[test]
fn parse_tech_stack_view() {
    let m = payments_model();
    let tsv = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::TechStack)
        .unwrap();
    assert_eq!(tsv.key, "TechStack");
    assert!(tsv.title.as_ref().unwrap().contains("Technology Stack"));
}

#[test]
fn parse_branching_strategy() {
    let m = payments_model();
    let branches: Vec<_> = m
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Branch)
        .collect();
    assert_eq!(branches.len(), 2);

    let trunk = branches.iter().find(|b| b.name == "main").unwrap();
    assert!(trunk.properties.contains_key("protection"));
    assert_eq!(
        trunk.properties.get("strategy").map(|s| s.as_str()),
        Some("trunk-based")
    );

    let feature = branches.iter().find(|b| b.name == "feature/*").unwrap();
    assert!(feature.properties.contains_key("branches-from"));
    assert!(feature.properties.contains_key("merges-into"));
}

#[test]
fn parse_branching_view() {
    let m = payments_model();
    let bv = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Branching)
        .unwrap();
    assert_eq!(bv.key, "Branching");
    assert_eq!(bv.scope.as_deref(), Some("trunk-based"));
}

#[test]
fn parse_data_model() {
    let m = payments_model();
    assert_eq!(m.data_entities.len(), 4);
    let txn = m
        .data_entities
        .iter()
        .find(|e| e.name == "Transaction")
        .unwrap();
    assert!(txn.fields.len() >= 5);
    assert_eq!(txn.fields[0].name, "id");
    assert_eq!(txn.fields[0].field_type, "UUID");
    assert!(txn.owner.is_some());

    assert_eq!(m.data_relations.len(), 3);
    assert!(m
        .data_relations
        .iter()
        .any(|r| r.from_entity == "Customer" && r.to_entity == "Transaction"));
}

#[test]
fn parse_trust_boundaries() {
    let m = payments_model();
    assert_eq!(m.trust_boundaries.len(), 4);
    let pci = m
        .trust_boundaries
        .iter()
        .find(|b| b.level == "pci")
        .unwrap();
    assert_eq!(pci.name, "PCI Data Zone");
    assert!(pci.members.iter().any(|m| m.contains("db")));
}

#[test]
fn parse_teams() {
    let m = payments_model();
    assert_eq!(m.teams.len(), 3);
    let platform = m.teams.iter().find(|t| t.name == "Platform Team").unwrap();
    assert!(platform.owns.len() >= 3);
    assert_eq!(platform.contact.as_deref(), Some("#platform-eng on Slack"));
}

#[test]
fn parse_components() {
    let m = payments_model();
    let components: Vec<_> = m
        .elements
        .values()
        .filter(|e| e.kind == ElementKind::Component)
        .collect();
    assert_eq!(components.len(), 4);

    let rest = m.elements.get("payments.api.rest").expect("REST component");
    assert_eq!(rest.name, "REST Controller");
    assert_eq!(rest.parent.as_deref(), Some("payments.api"));
    assert_eq!(rest.technology.as_deref(), Some("Actix-web"));
}

#[test]
fn parse_component_view() {
    let m = payments_model();
    let cv = m
        .views
        .iter()
        .find(|v| v.kind == ViewKind::Component)
        .unwrap();
    assert_eq!(cv.key, "APIComponents");
    assert_eq!(cv.scope.as_deref(), Some("payments.api"));
}

#[test]
fn parse_animation_frames() {
    let m = payments_model();
    let animated = m.views.iter().find(|v| v.key == "PaymentFlow").unwrap();
    assert!(!animated.animation.is_empty());
    assert_eq!(animated.animation.frames.len(), 5);

    let f1 = &animated.animation.frames[0];
    assert_eq!(f1.label, "Customer initiates payment");
    assert!(!f1.includes.is_empty());
    assert!(f1.notes.is_some());

    let f5 = &animated.animation.frames[4];
    assert_eq!(f5.label, "Complete payment flow");
    assert!(f5.include_all);
    assert!(!f5.highlights.is_empty());
    assert_eq!(f5.highlights[0].color.as_deref(), Some("#E65100"));
}

#[test]
fn parse_api_catalog() {
    let m = payments_model();
    assert_eq!(m.api_catalogs.len(), 2);
    let api = m
        .api_catalogs
        .iter()
        .find(|a| a.container.contains("api"))
        .unwrap();
    assert!(api.endpoints.len() >= 4);
    assert_eq!(api.endpoints[0].method, "POST");
    assert!(api.endpoints[0].path.contains("/payments"));
}

#[test]
fn parse_event_flows() {
    let m = payments_model();
    assert_eq!(m.event_flows.len(), 3);
    let flow = m
        .event_flows
        .iter()
        .find(|f| f.name == "payment-completed")
        .unwrap();
    assert!(flow.topic.is_some());
    assert!(!flow.publishers.is_empty());
    assert!(!flow.subscribers.is_empty());
}

#[test]
fn parse_env_config() {
    let m = payments_model();
    assert_eq!(m.env_configs.len(), 2);
    let prod = m
        .env_configs
        .iter()
        .find(|e| e.name == "production")
        .unwrap();
    assert!(prod.entries.iter().any(|e| e.key == "PAYMENT_GATEWAY"));
}

#[test]
fn parse_slos() {
    let m = payments_model();
    assert!(!m.slos.is_empty());
    let api_slo = m.slos.iter().find(|s| s.container.contains("api")).unwrap();
    assert!(api_slo.latency.is_some());
    assert!(api_slo.availability.is_some());
}

#[test]
fn parse_dependencies() {
    let m = payments_model();
    assert_eq!(m.dependencies.len(), 4);
    let stripe = m.dependencies.iter().find(|d| d.name == "Stripe").unwrap();
    assert_eq!(stripe.criticality, "critical");
    assert!(stripe.url.is_some());
}

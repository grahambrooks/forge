//! Top-level catalog/metadata blocks: tech-stack, data-model, trust-boundaries,
//! teams, apis, event-flows, env-config, slos, dependencies, docs.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_tech_stack(&mut self) -> Result<(), ParseError> {
        self.parse_braced("tech-stack", |this| {
            let kw = this.parse_ident()?;
            if kw == "category" {
                let cat_name = this.parse_string()?;
                let mut entries = Vec::new();
                this.parse_braced("tech-stack category", |this| {
                    let inner = this.parse_ident()?;
                    if inner == "tech" {
                        let tech_name = this.parse_string()?;
                        let mut entry = TechEntry {
                            name: tech_name,
                            version: None,
                            purpose: None,
                        };
                        if this.peek_after_ws() == Some('{') {
                            this.parse_braced("tech entry", |this| {
                                let prop = this.parse_ident()?;
                                match prop.as_str() {
                                    "version" => entry.version = Some(this.parse_string()?),
                                    "purpose" => entry.purpose = Some(this.parse_string()?),
                                    _ => {
                                        if this.peek_after_ws() == Some('"') {
                                            this.parse_string()?;
                                        }
                                    }
                                }
                                Ok(())
                            })?;
                        }
                        entries.push(entry);
                    } else if this.peek_after_ws() == Some('"') {
                        this.parse_string()?;
                    } else if this.peek_after_ws() == Some('{') {
                        this.skip_block()?;
                    }
                    Ok(())
                })?;
                this.model.tech_stack.push(TechCategory {
                    name: cat_name,
                    entries,
                });
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_data_model(&mut self) -> Result<(), ParseError> {
        self.parse_braced("data-model", |this| {
            let kw = this.parse_ident()?;
            if kw == "entity" {
                let name = this.parse_string()?;
                let mut entity = DataEntity {
                    name,
                    fields: Vec::new(),
                    owner: None,
                };
                if this.peek_after_ws() == Some('{') {
                    this.parse_braced("entity", |this| {
                        let prop = this.parse_ident()?;
                        match prop.as_str() {
                            "field" => {
                                let fname = this.parse_string()?;
                                let ftype = this.parse_string()?;
                                let mut constraints = Vec::new();
                                while this.peek_after_ws() == Some('"') {
                                    constraints.push(this.parse_string()?);
                                }
                                entity.fields.push(DataField {
                                    name: fname,
                                    field_type: ftype,
                                    constraints,
                                });
                            }
                            "owner" => {
                                let owner_ref = this.parse_ident()?;
                                entity.owner = Some(this.resolve_ref(&owner_ref));
                            }
                            _ => {
                                if this.peek_after_ws() == Some('"') {
                                    this.parse_string()?;
                                } else if this.peek_after_ws() == Some('{') {
                                    this.skip_block()?;
                                }
                            }
                        }
                        Ok(())
                    })?;
                }
                this.model.data_entities.push(entity);
            } else if kw == "relationship" {
                let from = this.parse_string()?;
                let to = this.parse_string()?;
                let mut label = String::new();
                let mut cardinality = "1:N".to_string();
                if this.peek_after_ws() == Some('{') {
                    this.parse_braced("relationship", |this| {
                        let p = this.parse_ident()?;
                        match p.as_str() {
                            "label" => label = this.parse_string()?,
                            "cardinality" => cardinality = this.parse_string()?,
                            _ => {
                                if this.peek_after_ws() == Some('"') {
                                    this.parse_string()?;
                                }
                            }
                        }
                        Ok(())
                    })?;
                }
                this.model.data_relations.push(DataRelation {
                    from_entity: from,
                    to_entity: to,
                    label,
                    cardinality,
                });
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_trust_boundaries(&mut self) -> Result<(), ParseError> {
        self.parse_braced("trust-boundaries", |this| {
            let kw = this.parse_ident()?;
            if kw == "boundary" {
                let name = this.parse_string()?;
                let mut boundary = TrustBoundary {
                    name,
                    level: "internal".into(),
                    members: Vec::new(),
                };
                this.parse_braced("boundary", |this| {
                    let prop = this.parse_ident()?;
                    match prop.as_str() {
                        "level" => boundary.level = this.parse_string()?,
                        "includes" => {
                            let member = this.parse_ident()?;
                            boundary.members.push(this.resolve_ref(&member));
                        }
                        _ => {
                            return Err(this.error(format!("unknown boundary keyword '{}'", prop)));
                        }
                    }
                    Ok(())
                })?;
                this.model.trust_boundaries.push(boundary);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_teams(&mut self) -> Result<(), ParseError> {
        self.parse_braced("teams", |this| {
            let kw = this.parse_ident()?;
            if kw == "team" {
                let name = this.parse_string()?;
                let mut team = Team {
                    name,
                    owns: Vec::new(),
                    contact: None,
                };
                this.parse_braced("team", |this| {
                    let prop = this.parse_ident()?;
                    match prop.as_str() {
                        "owns" => {
                            let target = this.parse_ident()?;
                            team.owns.push(this.resolve_ref(&target));
                        }
                        "contact" => team.contact = Some(this.parse_string()?),
                        _ => {
                            if this.peek_after_ws() == Some('"') {
                                this.parse_string()?;
                            } else if this.peek_after_ws() == Some('{') {
                                this.skip_block()?;
                            }
                        }
                    }
                    Ok(())
                })?;
                this.model.teams.push(team);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_apis(&mut self) -> Result<(), ParseError> {
        self.parse_braced("apis", |this| {
            let kw = this.parse_ident()?;
            if kw == "api" {
                let container_ref = this.parse_ident()?;
                let container = this.resolve_ref(&container_ref);
                let mut endpoints = Vec::new();
                this.parse_braced("api", |this| {
                    let inner = this.parse_ident()?;
                    if inner == "endpoint" {
                        // DSL v2: `endpoint "METHOD" "/path" { … }`.
                        // Structured two-string form instead of the v1
                        // `"METHOD /path"` split-on-space trick.
                        let method = this.parse_string()?;
                        let path = this.parse_string()?;
                        let mut ep = ApiEndpoint {
                            method,
                            path,
                            description: None,
                            request_body: None,
                            response: None,
                        };
                        if this.peek_after_ws() == Some('{') {
                            this.parse_braced("endpoint", |this| {
                                let p = this.parse_ident()?;
                                match p.as_str() {
                                    "description" => ep.description = Some(this.parse_string()?),
                                    "request" => ep.request_body = Some(this.parse_string()?),
                                    "response" => ep.response = Some(this.parse_string()?),
                                    _ => {
                                        if this.peek_after_ws() == Some('"') {
                                            this.parse_string()?;
                                        }
                                    }
                                }
                                Ok(())
                            })?;
                        }
                        endpoints.push(ep);
                    } else if this.peek_after_ws() == Some('{') {
                        this.skip_block()?;
                    }
                    Ok(())
                })?;
                this.model.api_catalogs.push(ApiCatalog {
                    container,
                    endpoints,
                });
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_event_flows(&mut self) -> Result<(), ParseError> {
        self.parse_braced("event-flows", |this| {
            let kw = this.parse_ident()?;
            if kw == "flow" {
                let name = this.parse_string()?;
                let mut flow = EventFlow {
                    name,
                    topic: None,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                    description: None,
                };
                this.parse_braced("flow", |this| {
                    let p = this.parse_ident()?;
                    match p.as_str() {
                        "topic" => flow.topic = Some(this.parse_string()?),
                        "description" => flow.description = Some(this.parse_string()?),
                        "publisher" => {
                            let r = this.parse_ident()?;
                            flow.publishers.push(this.resolve_ref(&r));
                        }
                        "subscriber" => {
                            let r = this.parse_ident()?;
                            flow.subscribers.push(this.resolve_ref(&r));
                        }
                        _ => {
                            if this.peek_after_ws() == Some('"') {
                                this.parse_string()?;
                            } else if this.peek_after_ws() == Some('{') {
                                this.skip_block()?;
                            }
                        }
                    }
                    Ok(())
                })?;
                this.model.event_flows.push(flow);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_env_config(&mut self) -> Result<(), ParseError> {
        self.parse_braced("env-config", |this| {
            let kw = this.parse_ident()?;
            if kw == "env" {
                let name = this.parse_string()?;
                let mut entries = Vec::new();
                this.parse_braced("env", |this| {
                    let key = this.parse_ident()?;
                    let value = this.parse_string()?;
                    entries.push(ConfigEntry { key, value });
                    Ok(())
                })?;
                this.model.env_configs.push(EnvConfig { name, entries });
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_slos(&mut self) -> Result<(), ParseError> {
        self.parse_braced("slos", |this| {
            let kw = this.parse_ident()?;
            if kw == "slo" {
                let container_ref = this.parse_ident()?;
                let container = this.resolve_ref(&container_ref);
                let mut slo = Slo {
                    container,
                    latency: None,
                    availability: None,
                    error_budget: None,
                };
                this.parse_braced("slo", |this| {
                    let p = this.parse_ident()?;
                    match p.as_str() {
                        "latency" => slo.latency = Some(this.parse_string()?),
                        "availability" => slo.availability = Some(this.parse_string()?),
                        "error-budget" => slo.error_budget = Some(this.parse_string()?),
                        _ => {
                            return Err(this.error(format!("unknown slo keyword '{}'", p)));
                        }
                    }
                    Ok(())
                })?;
                this.model.slos.push(slo);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_dependencies(&mut self) -> Result<(), ParseError> {
        self.parse_braced("dependencies", |this| {
            let kw = this.parse_ident()?;
            if kw == "dependency" {
                let name = this.parse_string()?;
                let mut dep = ExternalDependency {
                    name,
                    kind: "api".into(),
                    criticality: "medium".into(),
                    url: None,
                    description: None,
                };
                this.parse_braced("dependency", |this| {
                    let p = this.parse_ident()?;
                    match p.as_str() {
                        "kind" => dep.kind = this.parse_string()?,
                        "criticality" => dep.criticality = this.parse_string()?,
                        "url" => dep.url = Some(this.parse_string()?),
                        "description" => dep.description = Some(this.parse_string()?),
                        _ => {
                            return Err(this.error(format!("unknown dependency keyword '{}'", p)));
                        }
                    }
                    Ok(())
                })?;
                this.model.dependencies.push(dep);
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }

    pub(super) fn parse_docs(&mut self) -> Result<(), ParseError> {
        let mut order = 0;
        self.parse_braced("docs", |this| {
            let kw = this.parse_ident()?;
            if kw == "doc" {
                let title = this.parse_string()?;
                let path = this.parse_string()?;
                this.model.docs.push(Doc { title, path, order });
                order += 1;
            } else if this.peek_after_ws() == Some('"') {
                this.parse_string()?;
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
    }
}

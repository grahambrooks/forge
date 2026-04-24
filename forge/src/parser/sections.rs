//! Top-level catalog/metadata blocks: tech-stack, data-model, trust-boundaries,
//! teams, apis, event-flows, env-config, slos, dependencies, docs.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_tech_stack(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in tech-stack"));
            }
            let kw = self.parse_ident()?;
            if kw == "category" {
                let cat_name = self.parse_string()?;
                let mut entries = Vec::new();
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    if self.at_end() {
                        return Err(self.error("unexpected EOF in tech-stack category"));
                    }
                    let inner = self.parse_ident()?;
                    if inner == "tech" {
                        let tech_name = self.parse_string()?;
                        let mut entry = TechEntry {
                            name: tech_name,
                            version: None,
                            purpose: None,
                        };
                        if self.peek_after_ws() == Some('{') {
                            self.expect('{')?;
                            loop {
                                self.skip_ws();
                                if self.peek() == Some('}') {
                                    self.advance();
                                    break;
                                }
                                let prop = self.parse_ident()?;
                                match prop.as_str() {
                                    "version" => entry.version = Some(self.parse_string()?),
                                    "purpose" => entry.purpose = Some(self.parse_string()?),
                                    _ => {
                                        if self.peek_after_ws() == Some('"') {
                                            self.parse_string()?;
                                        }
                                    }
                                }
                            }
                        }
                        entries.push(entry);
                    } else if self.peek_after_ws() == Some('"') {
                        self.parse_string()?;
                    } else if self.peek_after_ws() == Some('{') {
                        self.skip_block()?;
                    }
                }
                self.model.tech_stack.push(TechCategory {
                    name: cat_name,
                    entries,
                });
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_data_model(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in data-model"));
            }
            let kw = self.parse_ident()?;
            if kw == "entity" {
                let name = self.parse_string()?;
                let mut entity = DataEntity {
                    name,
                    fields: Vec::new(),
                    owner: None,
                };
                if self.peek_after_ws() == Some('{') {
                    self.expect('{')?;
                    loop {
                        self.skip_ws();
                        if self.peek() == Some('}') {
                            self.advance();
                            break;
                        }
                        let prop = self.parse_ident()?;
                        match prop.as_str() {
                            "field" => {
                                let fname = self.parse_string()?;
                                let ftype = self.parse_string()?;
                                let mut constraints = Vec::new();
                                while self.peek_after_ws() == Some('"') {
                                    constraints.push(self.parse_string()?);
                                }
                                entity.fields.push(DataField {
                                    name: fname,
                                    field_type: ftype,
                                    constraints,
                                });
                            }
                            "owner" => {
                                let owner_ref = self.parse_ident()?;
                                entity.owner = Some(self.resolve_ref(&owner_ref));
                            }
                            _ => {
                                if self.peek_after_ws() == Some('"') {
                                    self.parse_string()?;
                                } else if self.peek_after_ws() == Some('{') {
                                    self.skip_block()?;
                                }
                            }
                        }
                    }
                }
                self.model.data_entities.push(entity);
            } else if kw == "relationship" {
                let from = self.parse_string()?;
                let to = self.parse_string()?;
                let mut label = String::new();
                let mut cardinality = "1:N".to_string();
                if self.peek_after_ws() == Some('{') {
                    self.expect('{')?;
                    loop {
                        self.skip_ws();
                        if self.peek() == Some('}') {
                            self.advance();
                            break;
                        }
                        let p = self.parse_ident()?;
                        match p.as_str() {
                            "label" => label = self.parse_string()?,
                            "cardinality" => cardinality = self.parse_string()?,
                            _ => {
                                if self.peek_after_ws() == Some('"') {
                                    self.parse_string()?;
                                }
                            }
                        }
                    }
                }
                self.model.data_relations.push(DataRelation {
                    from_entity: from,
                    to_entity: to,
                    label,
                    cardinality,
                });
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_trust_boundaries(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in trust-boundaries"));
            }
            let kw = self.parse_ident()?;
            if kw == "boundary" {
                let name = self.parse_string()?;
                let mut boundary = TrustBoundary {
                    name,
                    level: "internal".into(),
                    members: Vec::new(),
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let prop = self.parse_ident()?;
                    match prop.as_str() {
                        "level" => boundary.level = self.parse_string()?,
                        "includes" => {
                            let member = self.parse_ident()?;
                            boundary.members.push(self.resolve_ref(&member));
                        }
                        _ => {
                            return Err(self.error(format!("unknown boundary keyword '{}'", prop)));
                        }
                    }
                }
                self.model.trust_boundaries.push(boundary);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_teams(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in teams"));
            }
            let kw = self.parse_ident()?;
            if kw == "team" {
                let name = self.parse_string()?;
                let mut team = Team {
                    name,
                    owns: Vec::new(),
                    contact: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let prop = self.parse_ident()?;
                    match prop.as_str() {
                        "owns" => {
                            let target = self.parse_ident()?;
                            team.owns.push(self.resolve_ref(&target));
                        }
                        "contact" => team.contact = Some(self.parse_string()?),
                        _ => {
                            if self.peek_after_ws() == Some('"') {
                                self.parse_string()?;
                            } else if self.peek_after_ws() == Some('{') {
                                self.skip_block()?;
                            }
                        }
                    }
                }
                self.model.teams.push(team);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_apis(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in apis"));
            }
            let kw = self.parse_ident()?;
            if kw == "api" {
                let container_ref = self.parse_ident()?;
                let container = self.resolve_ref(&container_ref);
                let mut endpoints = Vec::new();
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let inner = self.parse_ident()?;
                    if inner == "endpoint" {
                        // DSL v2: `endpoint "METHOD" "/path" { … }`.
                        // Structured two-string form instead of the v1
                        // `"METHOD /path"` split-on-space trick.
                        let method = self.parse_string()?;
                        let path = self.parse_string()?;
                        let mut ep = ApiEndpoint {
                            method,
                            path,
                            description: None,
                            request_body: None,
                            response: None,
                        };
                        if self.peek_after_ws() == Some('{') {
                            self.expect('{')?;
                            loop {
                                self.skip_ws();
                                if self.peek() == Some('}') {
                                    self.advance();
                                    break;
                                }
                                let p = self.parse_ident()?;
                                match p.as_str() {
                                    "description" => ep.description = Some(self.parse_string()?),
                                    "request" => ep.request_body = Some(self.parse_string()?),
                                    "response" => ep.response = Some(self.parse_string()?),
                                    _ => {
                                        if self.peek_after_ws() == Some('"') {
                                            self.parse_string()?;
                                        }
                                    }
                                }
                            }
                        }
                        endpoints.push(ep);
                    } else if self.peek_after_ws() == Some('{') {
                        self.skip_block()?;
                    }
                }
                self.model.api_catalogs.push(ApiCatalog {
                    container,
                    endpoints,
                });
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_event_flows(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in event-flows"));
            }
            let kw = self.parse_ident()?;
            if kw == "flow" {
                let name = self.parse_string()?;
                let mut flow = EventFlow {
                    name,
                    topic: None,
                    publishers: Vec::new(),
                    subscribers: Vec::new(),
                    description: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let p = self.parse_ident()?;
                    match p.as_str() {
                        "topic" => flow.topic = Some(self.parse_string()?),
                        "description" => flow.description = Some(self.parse_string()?),
                        "publisher" => {
                            let r = self.parse_ident()?;
                            flow.publishers.push(self.resolve_ref(&r));
                        }
                        "subscriber" => {
                            let r = self.parse_ident()?;
                            flow.subscribers.push(self.resolve_ref(&r));
                        }
                        _ => {
                            if self.peek_after_ws() == Some('"') {
                                self.parse_string()?;
                            } else if self.peek_after_ws() == Some('{') {
                                self.skip_block()?;
                            }
                        }
                    }
                }
                self.model.event_flows.push(flow);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_env_config(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in env-config"));
            }
            let kw = self.parse_ident()?;
            if kw == "env" {
                let name = self.parse_string()?;
                let mut entries = Vec::new();
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let key = self.parse_ident()?;
                    let value = self.parse_string()?;
                    entries.push(ConfigEntry { key, value });
                }
                self.model.env_configs.push(EnvConfig { name, entries });
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_slos(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in slos"));
            }
            let kw = self.parse_ident()?;
            if kw == "slo" {
                let container_ref = self.parse_ident()?;
                let container = self.resolve_ref(&container_ref);
                let mut slo = Slo {
                    container,
                    latency: None,
                    availability: None,
                    error_budget: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let p = self.parse_ident()?;
                    match p.as_str() {
                        "latency" => slo.latency = Some(self.parse_string()?),
                        "availability" => slo.availability = Some(self.parse_string()?),
                        "error-budget" => slo.error_budget = Some(self.parse_string()?),
                        _ => {
                            return Err(self.error(format!("unknown slo keyword '{}'", p)));
                        }
                    }
                }
                self.model.slos.push(slo);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_dependencies(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in dependencies"));
            }
            let kw = self.parse_ident()?;
            if kw == "dependency" {
                let name = self.parse_string()?;
                let mut dep = ExternalDependency {
                    name,
                    kind: "api".into(),
                    criticality: "medium".into(),
                    url: None,
                    description: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    let p = self.parse_ident()?;
                    match p.as_str() {
                        "kind" => dep.kind = self.parse_string()?,
                        "criticality" => dep.criticality = self.parse_string()?,
                        "url" => dep.url = Some(self.parse_string()?),
                        "description" => dep.description = Some(self.parse_string()?),
                        _ => {
                            return Err(self.error(format!("unknown dependency keyword '{}'", p)));
                        }
                    }
                }
                self.model.dependencies.push(dep);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    pub(super) fn parse_docs(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        let mut order = 0;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in docs"));
            }
            let kw = self.parse_ident()?;
            if kw == "doc" {
                let title = self.parse_string()?;
                let path = self.parse_string()?;
                self.model.docs.push(Doc { title, path, order });
                order += 1;
            } else if self.peek_after_ws() == Some('"') {
                self.parse_string()?;
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }
}

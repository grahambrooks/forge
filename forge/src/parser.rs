//! Forge DSL parser — hand-written recursive descent.

use std::collections::HashMap;
use std::fmt;

use crate::model::*;

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.col, self.msg)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    text: Vec<char>,
    pos: usize,
    model: Model,
    scope: Vec<String>,
    id_map: HashMap<String, String>,
}

impl Parser {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            pos: 0,
            model: Model::default(),
            scope: Vec::new(),
            id_map: HashMap::new(),
        }
    }

    // ── Helpers ──

    fn line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, &ch) in self.text.iter().enumerate() {
            if i >= self.pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        let (line, col) = self.line_col();
        ParseError {
            msg: msg.into(),
            line,
            col,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.text.len() {
            let c = self.text[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    fn skip_ws(&mut self) {
        loop {
            while self.pos < self.text.len()
                && matches!(self.text[self.pos], ' ' | '\t' | '\r' | '\n')
            {
                self.pos += 1;
            }
            if self.pos + 1 < self.text.len()
                && self.text[self.pos] == '/'
                && self.text[self.pos + 1] == '/'
            {
                while self.pos < self.text.len() && self.text[self.pos] != '\n' {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), ParseError> {
        self.skip_ws();
        match self.advance() {
            Some(c) if c == ch => Ok(()),
            Some(c) => Err(self.error(format!("expected '{}', got '{}'", ch, c))),
            None => Err(self.error(format!("expected '{}', got EOF", ch))),
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.skip_ws();
        if self.peek() != Some('"') {
            return Err(self.error("expected quoted string"));
        }
        self.advance();
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(self.error("unterminated string")),
                Some('"') => return Ok(s),
                Some('\\') => {
                    if let Some(c2) = self.advance() {
                        s.push(c2);
                    }
                }
                Some(c) => s.push(c),
            }
        }
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.text.len() {
            let c = self.text[self.pos];
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '*' | '/') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.error("expected identifier"));
        }
        Ok(self.text[start..self.pos].iter().collect())
    }

    fn peek_after_ws(&mut self) -> Option<char> {
        let saved = self.pos;
        self.skip_ws();
        let c = self.peek();
        self.pos = saved;
        c
    }

    fn skip_block(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        let mut depth = 1;
        while depth > 0 {
            match self.advance() {
                None => return Err(self.error("unexpected EOF in block")),
                Some('{') => depth += 1,
                Some('}') => depth -= 1,
                Some('"') => loop {
                    match self.advance() {
                        None => return Err(self.error("unterminated string")),
                        Some('"') => break,
                        Some('\\') => {
                            self.advance();
                        }
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    fn scoped_id(&self, local: &str) -> String {
        if self.scope.is_empty() {
            local.to_string()
        } else {
            // Use the last scope entry (which is already a full path)
            format!("{}.{}", self.scope.last().unwrap(), local)
        }
    }

    fn resolve_ref(&self, name: &str) -> String {
        if let Some(full) = self.id_map.get(name) {
            return full.clone();
        }
        if !self.scope.is_empty() {
            let scoped = format!("{}.{}", self.scope.last().unwrap(), name);
            if self.model.elements.contains_key(&scoped) {
                return scoped;
            }
        }
        name.to_string()
    }

    // ── Top level ──

    pub fn parse(mut self) -> Result<Model, ParseError> {
        self.skip_ws();
        let kw = self.parse_ident()?;
        if kw != "forge" {
            return Err(self.error("expected 'forge'"));
        }
        self.model.name = self.parse_string()?;
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF"));
            }
            let kw = self.parse_ident()?;
            match kw.as_str() {
                "description" => {
                    self.model.description = self.parse_string()?;
                }
                "model" => self.parse_model()?,
                "process" => self.parse_process()?,
                "deployment" => self.parse_deployment()?,
                "tech-stack" => self.parse_tech_stack()?,
                "data-model" => self.parse_data_model()?,
                "trust-boundaries" => self.parse_trust_boundaries()?,
                "teams" => self.parse_teams()?,
                "apis" => self.parse_apis()?,
                "event-flows" => self.parse_event_flows()?,
                "env-config" => self.parse_env_config()?,
                "slos" => self.parse_slos()?,
                "dependencies" => self.parse_dependencies()?,
                "views" => self.parse_views()?,
                "docs" => self.parse_docs()?,
                _ => {
                    return Err(self.error(format!("unknown top-level block '{}'", kw)));
                }
            }
        }
        Ok(self.model)
    }

    // ── Model ──

    fn parse_model(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in model"));
            }
            self.parse_model_stmt()?;
        }
    }

    fn parse_model_stmt(&mut self) -> Result<(), ParseError> {
        let saved = self.pos;
        let first = self.parse_ident()?;
        self.skip_ws();

        if self.peek() == Some('=') {
            self.advance();
            let kind_str = self.parse_ident()?;
            let kind = match kind_str.as_str() {
                "person" => ElementKind::Person,
                "system" => ElementKind::System,
                "container" => ElementKind::Container,
                "component" => ElementKind::Component,
                _ => return Err(self.error(format!("unknown element kind '{}'", kind_str))),
            };
            let name = self.parse_string()?;
            let full_id = self.scoped_id(&first);
            let parent = if self.scope.is_empty() {
                None
            } else {
                Some(self.scope.last().unwrap().clone())
            };

            let mut el = Element::new(&full_id, kind, &name);
            el.parent = parent;
            self.id_map.insert(first.clone(), full_id.clone());

            if self.peek_after_ws() == Some('{') {
                self.expect('{')?;
                let scope_entry = if self.scope.is_empty() {
                    first.clone()
                } else {
                    full_id.clone()
                };
                self.scope.push(scope_entry);
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    if self.at_end() {
                        return Err(self.error("unexpected EOF in element"));
                    }

                    let saved2 = self.pos;
                    let inner = self.parse_ident()?;
                    self.skip_ws();

                    match inner.as_str() {
                        "description" => {
                            el.description = Some(self.parse_string()?);
                        }
                        "technology" => {
                            el.technology = Some(self.parse_string()?);
                        }
                        "tags" => {
                            while self.peek_after_ws() == Some('"') {
                                el.tags.push(self.parse_string()?);
                            }
                        }
                        "data-class" => {
                            while self.peek_after_ws() == Some('"') {
                                el.data_classes.push(self.parse_string()?);
                            }
                        }
                        _ => {
                            if self.peek() == Some('=') {
                                // Child element
                                self.model.add_element(el.clone());
                                self.pos = saved2;
                                self.parse_model_stmt()?;
                                if let Some(updated) = self.model.elements.get(&full_id) {
                                    el = updated.clone();
                                }
                                continue;
                            } else if self.peek() == Some('-') {
                                self.pos = saved2;
                                self.parse_relationship()?;
                                continue;
                            } else if self.peek_after_ws() == Some('"') {
                                self.parse_string()?;
                            } else if self.peek_after_ws() == Some('{') {
                                self.skip_block()?;
                            }
                        }
                    }
                }
                self.scope.pop();
            }
            self.model.add_element(el);
            return Ok(());
        }

        if self.peek() == Some('-') {
            self.pos = saved;
            self.parse_relationship()?;
            return Ok(());
        }

        if first == "description" {
            self.model.description = self.parse_string()?;
            return Ok(());
        }

        if self.peek_after_ws() == Some('"') {
            self.parse_string()?;
        } else if self.peek_after_ws() == Some('{') {
            self.skip_block()?;
        }
        Ok(())
    }

    fn parse_relationship(&mut self) -> Result<(), ParseError> {
        let frm_raw = self.parse_ident()?;
        self.skip_ws();
        self.expect('-')?;
        self.expect('>')?;
        let to_raw = self.parse_ident()?;
        let label = if self.peek_after_ws() == Some('"') {
            self.parse_string()?
        } else {
            String::new()
        };
        let technology = if self.peek_after_ws() == Some('"') {
            Some(self.parse_string()?)
        } else {
            None
        };

        let frm = self.resolve_ref(&frm_raw);
        let to = self.resolve_ref(&to_raw);
        self.model.add_relationship(Relationship {
            frm,
            to,
            label,
            technology,
            order: None,
        });
        Ok(())
    }

    // ── Process ──

    fn parse_process(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in process"));
            }
            self.parse_process_stmt()?;
        }
    }

    fn parse_process_stmt(&mut self) -> Result<(), ParseError> {
        let first = self.parse_ident()?;
        self.skip_ws();

        // DSL v2: every process-section element uses the same binding form
        // as model elements: `<id> = <kind> "Display Name" { ... }`.
        // Supported kinds: repository, strategy, pipeline.
        if self.peek() != Some('=') {
            return Err(self.error(format!(
                "expected '=' after process binding '{}', use `{} = <kind> \"Name\"`",
                first, first
            )));
        }
        self.advance();
        let kind_str = self.parse_ident()?;
        match kind_str.as_str() {
            "repository" => {
                let name = self.parse_string()?;
                let mut el = Element::new(&first, ElementKind::Repository, &name);
                self.id_map.insert(first.clone(), first.clone());
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
                            "url" => {
                                el.properties.insert("url".into(), self.parse_string()?);
                            }
                            "system" => {
                                let sys = self.parse_ident()?;
                                el.properties
                                    .insert("system".into(), self.resolve_ref(&sys));
                            }
                            _ => {
                                return Err(
                                    self.error(format!("unknown repository keyword '{}'", prop))
                                );
                            }
                        }
                    }
                }
                self.model.add_element(el);
                Ok(())
            }
            "strategy" => self.parse_strategy(first),
            "pipeline" => self.parse_pipeline(first),
            _ => Err(self.error(format!(
                "unknown process-section element kind '{}', expected repository/strategy/pipeline",
                kind_str
            ))),
        }
    }

    fn parse_strategy(&mut self, strategy_id: String) -> Result<(), ParseError> {
        let _display_name = self.parse_string()?;

        self.expect('{')?;
        while self.peek_after_ws() != Some('}') {
            if self.at_end() {
                return Err(self.error("unexpected EOF in strategy"));
            }
            let first = self.parse_ident()?;
            self.skip_ws();

            if self.peek() == Some('=') {
                self.advance();
                let kind_str = self.parse_ident()?;
                if kind_str == "branch" {
                    let branch_name = self.parse_string()?;
                    let branch_id = format!("{}.{}", strategy_id, first);
                    let mut el = Element::new(&branch_id, ElementKind::Branch, &branch_name);
                    el.parent = Some(strategy_id.clone());
                    el.properties.insert("strategy".into(), strategy_id.clone());
                    self.id_map.insert(first.clone(), branch_id.clone());

                    if self.peek_after_ws() == Some('{') {
                        self.expect('{')?;
                        loop {
                            self.skip_ws();
                            if self.peek() == Some('}') {
                                self.advance();
                                break;
                            }
                            if self.at_end() {
                                return Err(self.error("unexpected EOF in branch"));
                            }
                            let prop = self.parse_ident()?;
                            match prop.as_str() {
                                "protection" => {
                                    let mut protections = Vec::new();
                                    while self.peek_after_ws() == Some('"') {
                                        protections.push(self.parse_string()?);
                                    }
                                    el.properties
                                        .insert("protection".into(), protections.join(", "));
                                }
                                "branches-from" => {
                                    let target = self.parse_ident()?;
                                    let resolved = self.resolve_ref(&target);
                                    el.properties
                                        .insert("branches-from".into(), resolved.clone());
                                    self.model.add_relationship(Relationship {
                                        frm: resolved,
                                        to: branch_id.clone(),
                                        label: "branches from".into(),
                                        technology: None,
                                        order: None,
                                    });
                                }
                                "merges-into" => {
                                    let target = self.parse_ident()?;
                                    let resolved = self.resolve_ref(&target);
                                    el.properties.insert("merges-into".into(), resolved.clone());
                                    self.model.add_relationship(Relationship {
                                        frm: branch_id.clone(),
                                        to: resolved,
                                        label: "merges into".into(),
                                        technology: None,
                                        order: None,
                                    });
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
                    self.model.add_element(el);
                } else {
                    if self.peek_after_ws() == Some('"') {
                        self.parse_string()?;
                    }
                    if self.peek_after_ws() == Some('{') {
                        self.skip_block()?;
                    }
                }
            } else {
                if self.peek_after_ws() == Some('"') {
                    self.parse_string()?;
                }
                if self.peek_after_ws() == Some('{') {
                    self.skip_block()?;
                }
            }
        }
        self.expect('}')?;
        Ok(())
    }

    fn parse_pipeline(&mut self, pipeline_id: String) -> Result<(), ParseError> {
        let pipeline_name = self.parse_string()?;
        self.model.add_element(Element::new(
            &pipeline_id,
            ElementKind::Pipeline,
            &pipeline_name,
        ));
        self.id_map.insert(pipeline_id.clone(), pipeline_id.clone());

        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in pipeline"));
            }

            let first = self.parse_ident()?;
            self.skip_ws();

            if first == "triggers" {
                // `triggers <repo-ref> "event"`. Tokenised properly rather
                // than the line-scan hack of v1.
                let repo_ref = self.parse_ident()?;
                let _event = self.parse_string()?;
                let resolved = self.resolve_ref(&repo_ref);
                if let Some(el) = self.model.elements.get_mut(&pipeline_id) {
                    el.properties.insert("triggered_by".into(), resolved);
                }
                continue;
            }

            if self.peek() == Some('=') {
                self.advance();
                let kind_str = self.parse_ident()?;
                if kind_str == "stage" {
                    let stage_name = self.parse_string()?;
                    let stage_id = format!("{}.{}", pipeline_id, first);
                    let mut el = Element::new(&stage_id, ElementKind::Stage, &stage_name);
                    el.parent = Some(pipeline_id.clone());
                    self.id_map.insert(first.clone(), stage_id.clone());

                    if self.peek_after_ws() == Some('{') {
                        self.expect('{')?;
                        loop {
                            self.skip_ws();
                            if self.peek() == Some('}') {
                                self.advance();
                                break;
                            }
                            if self.at_end() {
                                return Err(self.error("unexpected EOF in stage"));
                            }
                            let prop = self.parse_ident()?;
                            match prop.as_str() {
                                "needs" => {
                                    let dep = self.parse_ident()?;
                                    let dep_full = self.resolve_ref(&dep);
                                    self.model.stage_links.push(StageLink {
                                        frm: dep_full,
                                        to: stage_id.clone(),
                                    });
                                }
                                "step" => {
                                    self.parse_string()?;
                                }
                                "environment" => {
                                    let env = self.parse_ident()?;
                                    el.properties.insert("environment".into(), env);
                                }
                                "gate" => {
                                    let gate_name = self.parse_string()?;
                                    let gate_id = format!("{}.gate", stage_id);
                                    let mut gate =
                                        Element::new(&gate_id, ElementKind::Gate, &gate_name);
                                    gate.parent = Some(stage_id.clone());
                                    if self.peek_after_ws() == Some('{') {
                                        self.expect('{')?;
                                        loop {
                                            self.skip_ws();
                                            if self.peek() == Some('}') {
                                                self.advance();
                                                break;
                                            }
                                            let gp = self.parse_ident()?;
                                            if self.peek_after_ws() == Some('"') {
                                                gate.properties.insert(gp, self.parse_string()?);
                                            }
                                        }
                                    }
                                    self.model.add_element(gate);
                                }
                                _ => {
                                    return Err(
                                        self.error(format!("unknown stage keyword '{}'", prop))
                                    );
                                }
                            }
                        }
                    }
                    self.model.add_element(el);
                } else {
                    return Err(self.error(format!(
                        "unknown pipeline element kind '{}', expected 'stage'",
                        kind_str
                    )));
                }
                continue;
            }

            return Err(self.error(format!("unknown pipeline keyword '{}'", first)));
        }
    }

    // ── Deployment ──

    fn parse_deployment(&mut self) -> Result<(), ParseError> {
        // DSL v2: `deployment <id> "Display Name" { node ... }`
        // The bare id is how views reference this environment.
        let env_id = self.parse_ident()?;
        let _display_name = self.parse_string()?;
        self.id_map.insert(env_id.clone(), env_id.clone());
        self.expect('{')?;
        while self.peek_after_ws() != Some('}') {
            if self.at_end() {
                return Err(self.error("unexpected EOF in deployment"));
            }
            let kw = self.parse_ident()?;
            if kw == "node" {
                self.parse_deployment_node(&env_id, &env_id)?;
            } else if self.peek_after_ws() == Some('"') {
                self.parse_string()?;
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
        self.expect('}')?;
        Ok(())
    }

    fn parse_deployment_node(&mut self, env_id: &str, parent_id: &str) -> Result<(), ParseError> {
        // DSL v2: `node <id> "Display Name" { ... }`. The bare id is
        // scoped under the environment and lets views (and `instance`
        // references) refer to nodes without string-lookup.
        let local = self.parse_ident()?;
        let node_name = self.parse_string()?;
        let node_id = format!("{}.{}", parent_id, local);

        let mut el = Element::new(&node_id, ElementKind::DeploymentNode, &node_name);
        el.parent = Some(parent_id.into());
        el.properties.insert("environment".into(), env_id.into());

        self.expect('{')?;
        // First pass: gather properties, then recurse for children
        while self.peek_after_ws() != Some('}') {
            if self.at_end() {
                return Err(self.error("unexpected EOF in deployment node"));
            }
            let kw = self.parse_ident()?;
            match kw.as_str() {
                "technology" => {
                    el.technology = Some(self.parse_string()?);
                }
                "description" => {
                    el.description = Some(self.parse_string()?);
                }
                "tags" => {
                    while self.peek_after_ws() == Some('"') {
                        el.tags.push(self.parse_string()?);
                    }
                }
                "instances" => {
                    let count = self.parse_ident()?;
                    el.properties.insert("instances".into(), count);
                }
                "node" => {
                    // Save element before recursing so parent exists
                    self.model.add_element(el.clone());
                    self.parse_deployment_node(env_id, &node_id)?;
                    // Refresh el from model (children may have been added)
                    if let Some(updated) = self.model.elements.get(&node_id) {
                        el = updated.clone();
                    }
                    continue;
                }
                "instance" => {
                    let container_ref = self.parse_ident()?;
                    let resolved = self.resolve_ref(&container_ref);
                    // Store instance refs as comma-separated in property
                    let existing = el
                        .properties
                        .entry("container_instances".into())
                        .or_default();
                    if !existing.is_empty() {
                        existing.push(',');
                    }
                    existing.push_str(&resolved);
                }
                _ => {
                    return Err(self.error(format!("unknown deployment-node keyword '{}'", kw)));
                }
            }
        }
        self.expect('}')?;
        self.model.add_element(el);
        Ok(())
    }

    // ── Views ──

    fn parse_views(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in views"));
            }

            let kind_str = self.parse_ident()?;
            // DSL v2: every view kind ends in `-view`. Scoped views take a
            // bare id; unscoped views take only a key. View bodies are
            // optional — a view with no properties other than the key is
            // valid (e.g. `tech-stack-view "TechStack"`).
            let (kind, needs_scope, default_layout) = match kind_str.as_str() {
                "system-context-view" => (ViewKind::SystemContext, true, AutoLayout::LeftRight),
                "container-view" => (ViewKind::Container, true, AutoLayout::TopBottom),
                "component-view" => (ViewKind::Component, true, AutoLayout::TopBottom),
                "pipeline-view" => (ViewKind::PipelineView, true, AutoLayout::LeftRight),
                "deployment-view" => (ViewKind::Deployment, true, AutoLayout::TopBottom),
                "branching-view" => (ViewKind::Branching, true, AutoLayout::LeftRight),
                "dynamic-view" => (ViewKind::Dynamic, true, AutoLayout::TopBottom),
                "tech-stack-view" => (ViewKind::TechStack, false, AutoLayout::TopBottom),
                "data-model-view" => (ViewKind::DataModel, false, AutoLayout::TopBottom),
                "trust-boundary-view" => {
                    (ViewKind::TrustBoundaryView, false, AutoLayout::TopBottom)
                }
                "team-view" => (ViewKind::TeamMap, false, AutoLayout::TopBottom),
                "api-catalog-view" => (ViewKind::ApiCatalogView, false, AutoLayout::TopBottom),
                "event-flow-view" => (ViewKind::EventFlowView, false, AutoLayout::TopBottom),
                "composite-view" => (ViewKind::Composite, false, AutoLayout::TopBottom),
                _ => {
                    return Err(self.error(format!("unknown view kind '{}'", kind_str)));
                }
            };

            let scope = if needs_scope {
                let scope_raw = self.parse_ident()?;
                Some(self.resolve_ref(&scope_raw))
            } else {
                None
            };
            let key = self.parse_string()?;
            let composite = if kind == ViewKind::Composite {
                Some(CompositeView {
                    cells: Vec::new(),
                    cols: 1,
                    rows: 1,
                    cell_size: (600, 400),
                })
            } else {
                None
            };
            let mut view = View {
                kind,
                key,
                scope,
                title: None,
                auto_layout: default_layout,
                include_all: false,
                animation: Animation::default(),
                composite,
            };
            // Optional body — v2 lets you omit `{}` for views with nothing
            // to configure beyond the key.
            if self.peek_after_ws() == Some('{') {
                self.parse_view_body(&mut view)?;
            }
            if view.kind == ViewKind::Composite {
                if let Some(comp) = view.composite.as_mut() {
                    if !comp.cells.is_empty() && comp.rows * comp.cols < comp.cells.len() as u32 {
                        comp.rows = comp.cells.len().div_ceil(comp.cols.max(1) as usize) as u32;
                    }
                }
            }
            self.model.views.push(view);
        }
    }

    fn parse_view_body(&mut self, view: &mut View) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in view"));
            }

            // Numbered relationship for dynamic views: `1. src -> dst "label"`.
            // Detection: leading ASCII digit followed (after optional whitespace)
            // by a `.`. Only meaningful inside dynamic views — we still parse
            // and drop the result elsewhere so a stray number doesn't blow up.
            if view.kind == ViewKind::Dynamic && self.peek().is_some_and(|c| c.is_ascii_digit()) {
                let order = self.parse_u32()?;
                self.skip_ws();
                self.expect('.')?;
                self.skip_ws();
                let src_raw = self.parse_ident()?;
                let src = self.resolve_ref(&src_raw);
                self.skip_ws();
                self.expect('-')?;
                self.expect('>')?;
                self.skip_ws();
                let dst_raw = self.parse_ident()?;
                let dst = self.resolve_ref(&dst_raw);
                let label = self.parse_string()?;
                let tech = if self.peek_after_ws() == Some('"') {
                    Some(self.parse_string()?)
                } else {
                    None
                };
                self.model.add_relationship(Relationship {
                    frm: src,
                    to: dst,
                    label,
                    technology: tech,
                    order: Some(order),
                });
                continue;
            }

            let prop = self.parse_ident()?;
            match prop.as_str() {
                "include" => {
                    self.skip_ws();
                    if self.peek() == Some('*') {
                        self.advance();
                        view.include_all = true;
                    } else {
                        // A list of element id references. Read until we
                        // hit something that isn't an ident-start so
                        // `include a b c` works as one line.
                        while self
                            .peek_after_ws()
                            .is_some_and(|c| c.is_alphanumeric() || c == '_')
                        {
                            self.parse_ident()?;
                        }
                    }
                }
                "auto-layout" => {
                    let d = self.parse_ident()?;
                    view.auto_layout = if d == "lr" {
                        AutoLayout::LeftRight
                    } else if d == "tb" {
                        AutoLayout::TopBottom
                    } else {
                        return Err(self.error(format!(
                            "unknown auto-layout direction '{}', expected 'lr' or 'tb'",
                            d
                        )));
                    };
                }
                "title" => {
                    view.title = Some(self.parse_string()?);
                }
                "animation" => {
                    self.parse_animation(&mut view.animation)?;
                }
                "grid" if view.kind == ViewKind::Composite => {
                    let cols = self.parse_u32()?;
                    self.skip_ws();
                    let rows = self.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cols = cols.max(1);
                        comp.rows = rows.max(1);
                    }
                }
                "cell-size" if view.kind == ViewKind::Composite => {
                    let w = self.parse_u32()?;
                    self.skip_ws();
                    let h = self.parse_u32()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cell_size = (w.max(100), h.max(100));
                    }
                }
                "cell" if view.kind == ViewKind::Composite => {
                    let key = self.parse_string()?;
                    if let Some(comp) = view.composite.as_mut() {
                        comp.cells.push(key);
                    }
                }
                _ => {
                    return Err(self.error(format!("unknown view keyword '{}'", prop)));
                }
            }
        }
    }

    /// Parse an unsigned integer literal. Used by dynamic view step numbers
    /// and composite view grid dimensions.
    fn parse_u32(&mut self) -> Result<u32, ParseError> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.text.len() && self.text[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected number"));
        }
        let s: String = self.text[start..self.pos].iter().collect();
        s.parse::<u32>().map_err(|_| self.error("invalid number"))
    }

    // ── Animation ──

    fn parse_animation(&mut self, anim: &mut Animation) -> Result<(), ParseError> {
        self.expect('{')?;
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                return Ok(());
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in animation"));
            }
            let kw = self.parse_ident()?;
            if kw == "frame" {
                let label = self.parse_string()?;
                let mut frame = AnimationFrame {
                    label,
                    includes: Vec::new(),
                    include_all: false,
                    highlights: Vec::new(),
                    states: Vec::new(),
                    notes: None,
                };
                self.expect('{')?;
                loop {
                    self.skip_ws();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    if self.at_end() {
                        return Err(self.error("unexpected EOF in frame"));
                    }
                    let prop = self.parse_ident()?;
                    match prop.as_str() {
                        "include" => {
                            self.skip_ws();
                            if self.peek() == Some('*') {
                                self.advance();
                                frame.include_all = true;
                            } else {
                                // Read element/relationship references
                                // Could be: "element", "el1 -> el2", or comma-separated
                                let ref1 = self.parse_ident()?;
                                self.skip_ws();
                                if self.peek() == Some('-') {
                                    // relationship: ref1 -> ref2
                                    self.advance(); // -
                                    self.expect('>')?;
                                    let ref2 = self.parse_ident()?;
                                    let r1 = self.resolve_ref(&ref1);
                                    let r2 = self.resolve_ref(&ref2);
                                    frame.includes.push(format!("{} -> {}", r1, r2));
                                } else {
                                    frame.includes.push(self.resolve_ref(&ref1));
                                }
                            }
                        }
                        "highlight" => {
                            let target = self.parse_ident()?;
                            let resolved = self.resolve_ref(&target);
                            let mut hl = FrameHighlight {
                                target: resolved,
                                color: None,
                                line_width: None,
                                label: None,
                            };
                            // Check for -> (relationship highlight)
                            self.skip_ws();
                            if self.peek() == Some('-') {
                                self.advance();
                                self.expect('>')?;
                                let t2 = self.parse_ident()?;
                                hl.target = format!("{} -> {}", hl.target, self.resolve_ref(&t2));
                                // May chain: a -> b -> c
                                self.skip_ws();
                                while self.peek() == Some('-') {
                                    self.advance();
                                    self.expect('>')?;
                                    let tn = self.parse_ident()?;
                                    hl.target
                                        .push_str(&format!(" -> {}", self.resolve_ref(&tn)));
                                }
                            }
                            if self.peek_after_ws() == Some('{') {
                                self.expect('{')?;
                                loop {
                                    self.skip_ws();
                                    if self.peek() == Some('}') {
                                        self.advance();
                                        break;
                                    }
                                    let hp = self.parse_ident()?;
                                    match hp.as_str() {
                                        "color" => hl.color = Some(self.parse_string()?),
                                        "line-width" => {
                                            let v = self.parse_ident()?;
                                            hl.line_width = v.parse().ok();
                                        }
                                        "label" => hl.label = Some(self.parse_string()?),
                                        _ => {
                                            return Err(self.error(format!(
                                                "unknown highlight keyword '{}'",
                                                hp
                                            )));
                                        }
                                    }
                                }
                            }
                            frame.highlights.push(hl);
                        }
                        "state" => {
                            let target = self.parse_ident()?;
                            let resolved = self.resolve_ref(&target);
                            let state_label = self.parse_string()?;
                            let mut state = FrameState {
                                target: resolved,
                                label: state_label,
                                color: None,
                                pulse: false,
                            };
                            if self.peek_after_ws() == Some('{') {
                                self.expect('{')?;
                                loop {
                                    self.skip_ws();
                                    if self.peek() == Some('}') {
                                        self.advance();
                                        break;
                                    }
                                    let sp = self.parse_ident()?;
                                    match sp.as_str() {
                                        "color" => state.color = Some(self.parse_string()?),
                                        "pulse" => {
                                            let v = self.parse_ident()?;
                                            state.pulse = v == "true";
                                        }
                                        _ => {
                                            if self.peek_after_ws() == Some('"') {
                                                self.parse_string()?;
                                            }
                                        }
                                    }
                                }
                            }
                            frame.states.push(state);
                        }
                        "notes" => {
                            frame.notes = Some(self.parse_string()?);
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
                anim.frames.push(frame);
            } else if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
    }

    // ── Tech Stack ──

    fn parse_tech_stack(&mut self) -> Result<(), ParseError> {
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

    // ── Data Model ──

    fn parse_data_model(&mut self) -> Result<(), ParseError> {
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

    // ── Trust Boundaries ──

    fn parse_trust_boundaries(&mut self) -> Result<(), ParseError> {
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

    // ── Teams ──

    fn parse_teams(&mut self) -> Result<(), ParseError> {
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

    // ── APIs ──

    fn parse_apis(&mut self) -> Result<(), ParseError> {
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

    // ── Event Flows ──

    fn parse_event_flows(&mut self) -> Result<(), ParseError> {
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

    // ── Environment Config ──

    fn parse_env_config(&mut self) -> Result<(), ParseError> {
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

    // ── SLOs ──

    fn parse_slos(&mut self) -> Result<(), ParseError> {
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

    // ── External Dependencies ──

    fn parse_dependencies(&mut self) -> Result<(), ParseError> {
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

    // ── Docs ──

    fn parse_docs(&mut self) -> Result<(), ParseError> {
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

pub fn parse(text: &str) -> Result<Model, ParseError> {
    let p = Parser::new(text);
    p.parse()
}

/// Parse with preprocessing (resolves !include, !fragment, !use, !if).
pub fn parse_with_preprocess(text: &str, base_dir: &std::path::Path) -> Result<Model, ParseError> {
    let processed = crate::preprocess::preprocess(text, base_dir).map_err(|e| ParseError {
        msg: e.msg,
        line: 0,
        col: 0,
    })?;
    parse(&processed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payments_model() -> Model {
        let text = include_str!("../examples/payments.forge");
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
}

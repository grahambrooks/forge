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
        ParseError { msg: msg.into(), line, col }
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

    fn peek_keyword(&mut self) -> Option<String> {
        let saved = self.pos;
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.text.len() {
            let c = self.text[self.pos];
            if c.is_alphanumeric() || matches!(c, '_' | '.') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let word: String = self.text[start..self.pos].iter().collect();
        self.pos = saved;
        if word.is_empty() { None } else { Some(word) }
    }

    fn skip_block(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        let mut depth = 1;
        while depth > 0 {
            match self.advance() {
                None => return Err(self.error("unexpected EOF in block")),
                Some('{') => depth += 1,
                Some('}') => depth -= 1,
                Some('"') => {
                    loop {
                        match self.advance() {
                            None => return Err(self.error("unterminated string")),
                            Some('"') => break,
                            Some('\\') => { self.advance(); }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn scoped_id(&self, local: &str) -> String {
        if self.scope.is_empty() {
            local.to_string()
        } else {
            format!("{}.{}", self.scope.join("."), local)
        }
    }

    fn resolve_ref(&self, name: &str) -> String {
        if let Some(full) = self.id_map.get(name) {
            return full.clone();
        }
        if !self.scope.is_empty() {
            let scoped = format!("{}.{}", self.scope.join("."), name);
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
                "views" => self.parse_views()?,
                "styles" => { self.skip_block()?; }
                _ => {
                    if self.peek_after_ws() == Some('{') {
                        self.skip_block()?;
                    } else if self.peek_after_ws() == Some('"') {
                        self.parse_string()?;
                    }
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
                Some(self.scope.join("."))
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

        if self.peek() == Some('=') {
            self.advance();
            let kind_str = self.parse_ident()?;
            if kind_str == "repository" {
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
                                el.properties.insert("system".into(), self.resolve_ref(&sys));
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
            return Ok(());
        }

        match first.as_str() {
            "strategy" => {
                self.parse_string()?;
                self.skip_block()?;
            }
            "pipeline" => {
                self.parse_pipeline()?;
            }
            _ => {
                if self.peek_after_ws() == Some('"') {
                    self.parse_string()?;
                }
                if self.peek_after_ws() == Some('{') {
                    self.skip_block()?;
                }
            }
        }
        Ok(())
    }

    fn parse_pipeline(&mut self) -> Result<(), ParseError> {
        let pipeline_name = self.parse_string()?;
        let pipeline_id = pipeline_name.replace(' ', "-").to_lowercase();
        self.model.add_element(Element::new(&pipeline_id, ElementKind::Pipeline, &pipeline_name));
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

            let _saved = self.pos;
            let first = self.parse_ident()?;
            self.skip_ws();

            if first == "triggers" {
                // Skip to end of line
                while self.pos < self.text.len() && self.text[self.pos] != '\n' {
                    self.pos += 1;
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
                                    let mut gate = Element::new(&gate_id, ElementKind::Gate, &gate_name);
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
                                "produces" => {
                                    self.parse_ident()?;
                                    self.parse_string()?;
                                    if self.peek_after_ws() == Some('{') {
                                        self.skip_block()?;
                                    }
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
                continue;
            }

            // Unknown
            if self.peek_after_ws() == Some('"') {
                self.parse_string()?;
            }
            if self.peek_after_ws() == Some('{') {
                self.skip_block()?;
            }
        }
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
            match kind_str.as_str() {
                "systemContext" => {
                    let scope_raw = self.parse_ident()?;
                    let scope = self.resolve_ref(&scope_raw);
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::SystemContext,
                        key,
                        scope: Some(scope),
                        title: None,
                        auto_layout: AutoLayout::LeftRight,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.views.push(view);
                }
                "container" => {
                    let scope_raw = self.parse_ident()?;
                    let scope = self.resolve_ref(&scope_raw);
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::Container,
                        key,
                        scope: Some(scope),
                        title: None,
                        auto_layout: AutoLayout::TopBottom,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.views.push(view);
                }
                "pipelineView" => {
                    let scope_raw = self.parse_string()?;
                    let scope_id = scope_raw.replace(' ', "-").to_lowercase();
                    let scope = self.resolve_ref(&scope_id);
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::PipelineView,
                        key,
                        scope: Some(scope),
                        title: None,
                        auto_layout: AutoLayout::LeftRight,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.views.push(view);
                }
                _ => {
                    // Skip unknown view types
                    loop {
                        let p = self.peek_after_ws();
                        if p == Some('"') {
                            self.parse_string()?;
                        } else if p.map_or(false, |c| c.is_alphanumeric()) {
                            self.parse_ident()?;
                        } else {
                            break;
                        }
                    }
                    if self.peek_after_ws() == Some('{') {
                        self.skip_block()?;
                    }
                }
            }
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
            let prop = self.parse_ident()?;
            match prop.as_str() {
                "include" => {
                    self.skip_ws();
                    if self.peek() == Some('*') {
                        self.advance();
                        view.include_all = true;
                    } else {
                        self.parse_ident()?;
                    }
                }
                "autoLayout" => {
                    let d = self.parse_ident()?;
                    view.auto_layout = if d == "lr" {
                        AutoLayout::LeftRight
                    } else {
                        AutoLayout::TopBottom
                    };
                }
                "title" => {
                    view.title = Some(self.parse_string()?);
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
}

pub fn parse(text: &str) -> Result<Model, ParseError> {
    let p = Parser::new(text);
    p.parse()
}

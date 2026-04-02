/// Forge DSL parser — hand-written recursive descent.
///
/// Parses a subset of the Forge DSL sufficient for the prototype:
/// - forge "Name" { model { ... } process { ... } views { ... } }
/// - person, system, container (with technology, description, tags)
/// - relationships: a -> b "label" "tech"
/// - pipeline, stage (with needs, step, gate, produces)
/// - views: systemContext, container, pipelineView

use crate::model::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
    model: Model,
    /// Stack of identifier scopes for building dotted ids: ["payments", "api"]
    scope: Vec<String>,
    /// Map from short identifiers to full dotted ids (e.g. "api" -> "payments.api")
    id_map: HashMap<String, String>,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            model: Model::new(),
            scope: Vec::new(),
            id_map: HashMap::new(),
        }
    }

    fn current_line_col(&self) -> (usize, usize) {
        let consumed = &self.chars[..self.pos.min(self.chars.len())];
        let line = consumed.iter().filter(|&&c| c == '\n').count() + 1;
        let col = consumed.iter().rev().take_while(|&&c| c != '\n').count() + 1;
        (line, col)
    }

    fn error(&self, msg: &str) -> ParseError {
        let (line, col) = self.current_line_col();
        ParseError { message: msg.to_string(), line, col }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }
            // Skip // line comments
            if self.pos + 1 < self.chars.len()
                && self.chars[self.pos] == '/'
                && self.chars[self.pos + 1] == '/'
            {
                while self.pos < self.chars.len() && self.chars[self.pos] != '\n' {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        self.skip_ws_and_comments();
        match self.advance() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(self.error(&format!("expected '{}', got '{}'", expected, c))),
            None => Err(self.error(&format!("expected '{}', got EOF", expected))),
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.skip_ws_and_comments();
        if self.peek() != Some('"') {
            return Err(self.error("expected quoted string"));
        }
        self.advance(); // consume opening "
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(s),
                Some('\\') => {
                    if let Some(c) = self.advance() {
                        s.push(c);
                    }
                }
                Some(c) => s.push(c),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_alphanumeric()
                || self.chars[self.pos] == '_'
                || self.chars[self.pos] == '-'
                || self.chars[self.pos] == '.'
                || self.chars[self.pos] == '*'
                || self.chars[self.pos] == '/')
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected identifier"));
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn peek_keyword(&mut self) -> Option<String> {
        let saved = self.pos;
        self.skip_ws_and_comments();
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_alphanumeric()
                || self.chars[self.pos] == '_'
                || self.chars[self.pos] == '.')
        {
            self.pos += 1;
        }
        let word: String = self.chars[start..self.pos].iter().collect();
        self.pos = saved;
        if word.is_empty() {
            None
        } else {
            Some(word)
        }
    }

    fn peek_char_skip_ws(&mut self) -> Option<char> {
        let saved = self.pos;
        self.skip_ws_and_comments();
        let c = self.peek();
        self.pos = saved;
        c
    }

    fn scoped_id(&self, local: &str) -> String {
        if self.scope.is_empty() {
            local.to_string()
        } else {
            format!("{}.{}", self.scope.join("."), local)
        }
    }

    // ───── Top-level ─────

    fn parse_forge(&mut self) -> Result<(), ParseError> {
        self.skip_ws_and_comments();
        let kw = self.parse_identifier()?;
        if kw != "forge" {
            return Err(self.error("expected 'forge'"));
        }
        self.model.name = self.parse_string()?;
        self.expect_char('{')?;

        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in forge block"));
            }
            let kw = self.parse_identifier()?;
            match kw.as_str() {
                "description" => {
                    self.model.description = self.parse_string()?;
                }
                "model" => self.parse_model()?,
                "process" => self.parse_process()?,
                "views" => self.parse_views()?,
                "styles" => self.skip_block()?,  // styles: skip for now
                _ => {
                    // Try to skip unknown blocks or properties
                    if self.peek_char_skip_ws() == Some('{') {
                        self.skip_block()?;
                    } else if self.peek_char_skip_ws() == Some('"') {
                        let _ = self.parse_string()?;
                    }
                }
            }
        }
        Ok(())
    }

    // ───── Model block ─────

    fn parse_model(&mut self) -> Result<(), ParseError> {
        self.expect_char('{')?;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in model block"));
            }
            self.parse_model_statement()?;
        }
        Ok(())
    }

    fn parse_model_statement(&mut self) -> Result<(), ParseError> {
        let saved = self.pos;
        let first = self.parse_identifier()?;

        self.skip_ws_and_comments();

        // Check for assignment: id = kind "Name" { ... }
        if self.peek() == Some('=') {
            self.advance(); // consume '='
            let kind_str = self.parse_identifier()?;
            let kind = match kind_str.as_str() {
                "person" => ElementKind::Person,
                "system" => ElementKind::System,
                "container" => ElementKind::Container,
                "component" => ElementKind::Component,
                _ => return Err(self.error(&format!("unknown element kind '{}'", kind_str))),
            };
            let name = self.parse_string()?;
            let full_id = self.scoped_id(&first);
            let mut el = Element {
                id: full_id.clone(),
                kind,
                name,
                description: None,
                technology: None,
                tags: Vec::new(),
                parent: self.scope.last().map(|s| {
                    if self.scope.len() == 1 {
                        s.clone()
                    } else {
                        self.scope.join(".")
                    }
                }),
                properties: HashMap::new(),
                children: Vec::new(),
            };
            self.id_map.insert(first.clone(), full_id.clone());

            if self.peek_char_skip_ws() == Some('{') {
                self.expect_char('{')?;
                self.scope.push(if self.scope.is_empty() {
                    first.clone()
                } else {
                    full_id.clone()
                });
                loop {
                    self.skip_ws_and_comments();
                    if self.peek() == Some('}') {
                        self.advance();
                        break;
                    }
                    if self.at_end() {
                        return Err(self.error("unexpected EOF in element block"));
                    }
                    // Could be a property, child element, or relationship
                    let saved2 = self.pos;
                    let inner = self.parse_identifier()?;
                    self.skip_ws_and_comments();

                    if inner == "description" {
                        el.description = Some(self.parse_string()?);
                    } else if inner == "technology" {
                        el.technology = Some(self.parse_string()?);
                    } else if inner == "tags" {
                        while self.peek_char_skip_ws() == Some('"') {
                            el.tags.push(self.parse_string()?);
                        }
                    } else if self.peek() == Some('=') {
                        // Child element assignment — put element first, then parse child
                        self.model.add_element(el.clone());
                        self.pos = saved2;
                        self.parse_model_statement()?;
                        // Re-read updated element to pick up children
                        if let Some(updated) = self.model.elements.get(&full_id) {
                            el = updated.clone();
                        }
                        continue;
                    } else if self.peek() == Some('-') {
                        // Relationship: inner -> target "label" "tech"
                        self.pos = saved2;
                        self.parse_relationship()?;
                        continue;
                    } else {
                        // Unknown property — try to skip value
                        if self.peek_char_skip_ws() == Some('"') {
                            let _ = self.parse_string()?;
                        } else if self.peek_char_skip_ws() == Some('{') {
                            self.skip_block()?;
                        }
                    }
                }
                self.scope.pop();
            }
            self.model.add_element(el);
            return Ok(());
        }

        // Check for relationship: first -> target ...
        if self.peek() == Some('-') {
            self.pos = saved;
            self.parse_relationship()?;
            return Ok(());
        }

        // Keyword like "description"
        if first == "description" {
            self.model.description = self.parse_string()?;
            return Ok(());
        }

        // Unknown — skip
        if self.peek_char_skip_ws() == Some('"') {
            let _ = self.parse_string()?;
        } else if self.peek_char_skip_ws() == Some('{') {
            self.skip_block()?;
        }
        Ok(())
    }

    fn resolve_ref(&self, name: &str) -> String {
        // Try the id_map first (short name -> full dotted id)
        if let Some(full) = self.id_map.get(name) {
            return full.clone();
        }
        // Try scoped
        if !self.scope.is_empty() {
            let scoped = format!("{}.{}", self.scope.join("."), name);
            if self.model.elements.contains_key(&scoped) {
                return scoped;
            }
        }
        // As-is
        name.to_string()
    }

    fn parse_relationship(&mut self) -> Result<(), ParseError> {
        let from_raw = self.parse_identifier()?;
        self.skip_ws_and_comments();
        self.expect_char('-')?;
        self.expect_char('>')?;
        let to_raw = self.parse_identifier()?;
        let label = if self.peek_char_skip_ws() == Some('"') {
            self.parse_string()?
        } else {
            String::new()
        };
        let tech = if self.peek_char_skip_ws() == Some('"') {
            Some(self.parse_string()?)
        } else {
            None
        };

        let from = self.resolve_ref(&from_raw);
        let to = self.resolve_ref(&to_raw);

        self.model.add_relationship(Relationship {
            from,
            to,
            label,
            technology: tech,
        });
        Ok(())
    }

    // ───── Process block ─────

    fn parse_process(&mut self) -> Result<(), ParseError> {
        self.expect_char('{')?;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err(self.error("unexpected EOF in process block"));
            }
            self.parse_process_statement()?;
        }
        Ok(())
    }

    fn parse_process_statement(&mut self) -> Result<(), ParseError> {
        let saved = self.pos;
        let first = self.parse_identifier()?;
        self.skip_ws_and_comments();

        // Assignment: id = kind "Name" { ... }
        if self.peek() == Some('=') {
            self.advance();
            let kind_str = self.parse_identifier()?;
            match kind_str.as_str() {
                "repository" => {
                    let name = self.parse_string()?;
                    let full_id = first.clone();
                    let mut el = Element {
                        id: full_id.clone(),
                        kind: ElementKind::Repository,
                        name,
                        description: None,
                        technology: None,
                        tags: Vec::new(),
                        parent: None,
                        properties: HashMap::new(),
                        children: Vec::new(),
                    };
                    self.id_map.insert(first.clone(), full_id.clone());
                    if self.peek_char_skip_ws() == Some('{') {
                        self.expect_char('{')?;
                        loop {
                            self.skip_ws_and_comments();
                            if self.peek() == Some('}') { self.advance(); break; }
                            let prop = self.parse_identifier()?;
                            match prop.as_str() {
                                "url" => { el.properties.insert("url".into(), self.parse_string()?); }
                                "system" => { let sys = self.parse_identifier()?; el.properties.insert("system".into(), self.resolve_ref(&sys)); }
                                _ => { if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; } else if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; } }
                            }
                        }
                    }
                    self.model.add_element(el);
                }
                _ => {
                    // Unknown process element
                    if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                    if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
                }
            }
            return Ok(());
        }

        // Keywords
        match first.as_str() {
            "strategy" => {
                let _ = self.parse_string()?; // strategy name
                self.skip_block()?; // skip for prototype
            }
            "pipeline" => {
                self.parse_pipeline()?;
            }
            _ => {
                self.pos = saved;
                // Try to skip
                let _ = self.parse_identifier()?;
                if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
            }
        }
        Ok(())
    }

    fn parse_pipeline(&mut self) -> Result<(), ParseError> {
        let pipeline_name = self.parse_string()?;
        let pipeline_id = pipeline_name.replace(' ', "-").to_lowercase();
        let pipeline_el = Element {
            id: pipeline_id.clone(),
            kind: ElementKind::Pipeline,
            name: pipeline_name,
            description: None,
            technology: None,
            tags: Vec::new(),
            parent: None,
            properties: HashMap::new(),
            children: Vec::new(),
        };
        self.model.add_element(pipeline_el);
        self.id_map.insert(pipeline_id.clone(), pipeline_id.clone());

        self.expect_char('{')?;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') { self.advance(); break; }
            if self.at_end() { return Err(self.error("unexpected EOF in pipeline block")); }

            let saved = self.pos;
            let first = self.parse_identifier()?;
            self.skip_ws_and_comments();

            if first == "triggers" {
                // triggers repo.main on "push" — skip for now
                while self.peek_char_skip_ws() != Some('}') && self.peek_char_skip_ws() != None {
                    let saved2 = self.pos;
                    self.skip_ws_and_comments();
                    // Check if next is a new statement (identifier followed by = or known keyword)
                    if let Some(c) = self.peek() {
                        if c == '\n' || c == '\r' {
                            self.advance();
                            // Check if next line starts a new statement
                            let saved3 = self.pos;
                            self.skip_ws_and_comments();
                            if let Some(next_kw) = self.peek_keyword() {
                                if next_kw == "triggers" || self.is_stage_assignment() {
                                    self.pos = saved3;
                                    break;
                                }
                            }
                            self.pos = saved3;
                            break;
                        }
                    }
                    self.advance();
                }
                continue;
            }

            // Stage assignment: id = stage "Name" { ... }
            if self.peek() == Some('=') {
                self.advance();
                let kind_str = self.parse_identifier()?;
                if kind_str == "stage" {
                    let stage_name = self.parse_string()?;
                    let stage_id = format!("{}.{}", pipeline_id, first);
                    let mut stage_el = Element {
                        id: stage_id.clone(),
                        kind: ElementKind::Stage,
                        name: stage_name,
                        description: None,
                        technology: None,
                        tags: Vec::new(),
                        parent: Some(pipeline_id.clone()),
                        properties: HashMap::new(),
                        children: Vec::new(),
                    };
                    self.id_map.insert(first.clone(), stage_id.clone());

                    if self.peek_char_skip_ws() == Some('{') {
                        self.expect_char('{')?;
                        loop {
                            self.skip_ws_and_comments();
                            if self.peek() == Some('}') { self.advance(); break; }
                            if self.at_end() { return Err(self.error("unexpected EOF in stage block")); }
                            let prop = self.parse_identifier()?;
                            match prop.as_str() {
                                "needs" => {
                                    let dep = self.parse_identifier()?;
                                    let dep_full = self.resolve_ref(&dep);
                                    self.model.stage_links.push(StageLink {
                                        from: dep_full,
                                        to: stage_id.clone(),
                                    });
                                }
                                "step" => { let _ = self.parse_string()?; }
                                "environment" => {
                                    let env_name = self.parse_identifier()?;
                                    stage_el.properties.insert("environment".into(), env_name);
                                }
                                "gate" => {
                                    let gate_name = self.parse_string()?;
                                    let gate_id = format!("{}.gate", stage_id);
                                    let mut gate_el = Element {
                                        id: gate_id.clone(),
                                        kind: ElementKind::Gate,
                                        name: gate_name,
                                        description: None,
                                        technology: None,
                                        tags: Vec::new(),
                                        parent: Some(stage_id.clone()),
                                        properties: HashMap::new(),
                                        children: Vec::new(),
                                    };
                                    if self.peek_char_skip_ws() == Some('{') {
                                        self.expect_char('{')?;
                                        loop {
                                            self.skip_ws_and_comments();
                                            if self.peek() == Some('}') { self.advance(); break; }
                                            let gprop = self.parse_identifier()?;
                                            if self.peek_char_skip_ws() == Some('"') {
                                                gate_el.properties.insert(gprop, self.parse_string()?);
                                            }
                                        }
                                    }
                                    self.model.add_element(gate_el);
                                }
                                "produces" => {
                                    // produces artifact "name" { ... }
                                    let _ = self.parse_identifier()?; // "artifact"
                                    let _ = self.parse_string()?;
                                    if self.peek_char_skip_ws() == Some('{') {
                                        self.skip_block()?;
                                    }
                                }
                                _ => {
                                    if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                                    else if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
                                }
                            }
                        }
                    }
                    self.model.add_element(stage_el);
                } else {
                    // Unknown kind
                    if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                    if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
                }
                continue;
            }

            // Unknown
            self.pos = saved;
            let _ = self.parse_identifier()?;
            if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
            if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
        }
        Ok(())
    }

    fn is_stage_assignment(&mut self) -> bool {
        let saved = self.pos;
        self.skip_ws_and_comments();
        let _ = self.parse_identifier(); // id
        self.skip_ws_and_comments();
        let result = self.peek() == Some('=');
        self.pos = saved;
        result
    }

    // ───── Views block ─────

    fn parse_views(&mut self) -> Result<(), ParseError> {
        self.expect_char('{')?;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') { self.advance(); break; }
            if self.at_end() { return Err(self.error("unexpected EOF in views block")); }

            let kind_str = self.parse_identifier()?;
            match kind_str.as_str() {
                "systemContext" => {
                    let scope_raw = self.parse_identifier()?;
                    let scope = self.resolve_ref(&scope_raw);
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::SystemContext,
                        scope: Some(scope),
                        key,
                        title: None,
                        auto_layout: AutoLayout::LeftRight,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.add_view(view);
                }
                "container" => {
                    let scope_raw = self.parse_identifier()?;
                    let scope = self.resolve_ref(&scope_raw);
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::Container,
                        scope: Some(scope),
                        key,
                        title: None,
                        auto_layout: AutoLayout::TopBottom,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.add_view(view);
                }
                "pipelineView" => {
                    let scope_raw = self.parse_string()?;
                    let scope = self.resolve_ref(&scope_raw.replace(' ', "-").to_lowercase());
                    let key = self.parse_string()?;
                    let mut view = View {
                        kind: ViewKind::PipelineView,
                        scope: Some(scope),
                        key,
                        title: None,
                        auto_layout: AutoLayout::LeftRight,
                        include_all: false,
                    };
                    self.parse_view_body(&mut view)?;
                    self.model.add_view(view);
                }
                _ => {
                    // Unknown view type — skip
                    while self.peek_char_skip_ws() == Some('"') || self.peek_char_skip_ws().map_or(false, |c| c.is_alphanumeric()) {
                        if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                        else { let _ = self.parse_identifier()?; }
                    }
                    if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
                }
            }
        }
        Ok(())
    }

    fn parse_view_body(&mut self, view: &mut View) -> Result<(), ParseError> {
        self.expect_char('{')?;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') { self.advance(); break; }
            if self.at_end() { return Err(self.error("unexpected EOF in view block")); }
            let prop = self.parse_identifier()?;
            match prop.as_str() {
                "include" => {
                    self.skip_ws_and_comments();
                    if self.peek() == Some('*') {
                        self.advance();
                        view.include_all = true;
                    } else {
                        let _ = self.parse_identifier()?;
                    }
                }
                "autoLayout" => {
                    let dir = self.parse_identifier()?;
                    view.auto_layout = match dir.as_str() {
                        "lr" => AutoLayout::LeftRight,
                        "tb" => AutoLayout::TopBottom,
                        _ => AutoLayout::TopBottom,
                    };
                }
                "title" => {
                    view.title = Some(self.parse_string()?);
                }
                _ => {
                    if self.peek_char_skip_ws() == Some('"') { let _ = self.parse_string()?; }
                    else if self.peek_char_skip_ws() == Some('{') { self.skip_block()?; }
                }
            }
        }
        Ok(())
    }

    // ───── Utilities ─────

    fn skip_block(&mut self) -> Result<(), ParseError> {
        self.expect_char('{')?;
        let mut depth = 1;
        while depth > 0 {
            match self.advance() {
                Some('{') => depth += 1,
                Some('}') => depth -= 1,
                Some('"') => {
                    // Skip string contents
                    loop {
                        match self.advance() {
                            Some('"') => break,
                            Some('\\') => { self.advance(); }
                            None => return Err(self.error("unterminated string in skipped block")),
                            _ => {}
                        }
                    }
                }
                None => return Err(self.error("unexpected EOF while skipping block")),
                _ => {}
            }
        }
        Ok(())
    }
}

pub fn parse(input: &str) -> Result<Model, ParseError> {
    let mut parser = Parser::new(input);
    parser.parse_forge()?;
    Ok(parser.model)
}

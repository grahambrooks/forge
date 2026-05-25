//! Forge DSL parser — hand-written recursive descent.
//!
//! The parser is split into sub-modules by DSL section. This file holds the
//! shared [`Parser`] state and the top-level dispatch; each sub-module
//! contributes `impl Parser { ... }` methods for its concern.

use std::collections::HashMap;
use std::fmt;

use crate::model::*;

mod deployment;
mod lexer;
mod model_stmts;
mod process;
mod sections;
mod views;

#[cfg(test)]
mod tests;

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
    pub(super) text: Vec<char>,
    pub(super) pos: usize,
    pub(super) model: Model,
    pub(super) scope: Vec<String>,
    pub(super) id_map: HashMap<String, String>,
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

    pub fn parse(mut self) -> Result<Model, ParseError> {
        self.skip_ws();
        let kw = self.parse_ident()?;
        if kw != "forge" {
            return Err(self.error("expected 'forge'"));
        }
        self.model.name = self.parse_string()?;
        self.parse_braced("forge", |this| {
            let kw = this.parse_ident()?;
            match kw.as_str() {
                "description" => {
                    this.model.description = this.parse_string()?;
                }
                "model" => this.parse_model()?,
                "process" => this.parse_process()?,
                "deployment" => this.parse_deployment()?,
                "tech-stack" => this.parse_tech_stack()?,
                "data-model" => this.parse_data_model()?,
                "trust-boundaries" => this.parse_trust_boundaries()?,
                "teams" => this.parse_teams()?,
                "apis" => this.parse_apis()?,
                "event-flows" => this.parse_event_flows()?,
                "env-config" => this.parse_env_config()?,
                "slos" => this.parse_slos()?,
                "dependencies" => this.parse_dependencies()?,
                "views" => this.parse_views()?,
                "docs" => this.parse_docs()?,
                _ => {
                    return Err(this.error(format!("unknown top-level block '{}'", kw)));
                }
            }
            Ok(())
        })?;
        Ok(self.model)
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

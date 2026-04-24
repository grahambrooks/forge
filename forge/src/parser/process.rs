//! `process { ... }` block: repository, strategy, and pipeline declarations.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_process(&mut self) -> Result<(), ParseError> {
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

            if first == "tags" {
                // Quoted-string list; attach to the pipeline element.
                let mut tags: Vec<String> = Vec::new();
                while self.peek_after_ws() == Some('"') {
                    tags.push(self.parse_string()?);
                }
                if let Some(el) = self.model.elements.get_mut(&pipeline_id) {
                    el.tags.extend(tags);
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
                                "tags" => {
                                    while self.peek_after_ws() == Some('"') {
                                        el.tags.push(self.parse_string()?);
                                    }
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
}

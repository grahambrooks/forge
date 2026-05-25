//! `process { ... }` block: repository, strategy, and pipeline declarations.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_process(&mut self) -> Result<(), ParseError> {
        self.parse_braced("process", |this| this.parse_process_stmt())
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
                    self.parse_braced("repository", |this| {
                        let prop = this.parse_ident()?;
                        match prop.as_str() {
                            "url" => {
                                el.properties.insert("url".into(), this.parse_string()?);
                            }
                            "system" => {
                                let sys = this.parse_ident()?;
                                el.properties
                                    .insert("system".into(), this.resolve_ref(&sys));
                            }
                            _ => {
                                return Err(
                                    this.error(format!("unknown repository keyword '{}'", prop))
                                );
                            }
                        }
                        Ok(())
                    })?;
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

        self.parse_braced("strategy", |this| {
            let first = this.parse_ident()?;
            this.skip_ws();

            if this.peek() == Some('=') {
                this.advance();
                let kind_str = this.parse_ident()?;
                if kind_str == "branch" {
                    let branch_name = this.parse_string()?;
                    let branch_id = format!("{}.{}", strategy_id, first);
                    let mut el = Element::new(&branch_id, ElementKind::Branch, &branch_name);
                    el.parent = Some(strategy_id.clone());
                    el.properties.insert("strategy".into(), strategy_id.clone());
                    this.id_map.insert(first.clone(), branch_id.clone());

                    if this.peek_after_ws() == Some('{') {
                        let branch_id_for_body = branch_id.clone();
                        this.parse_braced("branch", |this| {
                            let prop = this.parse_ident()?;
                            match prop.as_str() {
                                "protection" => {
                                    let mut protections = Vec::new();
                                    while this.peek_after_ws() == Some('"') {
                                        protections.push(this.parse_string()?);
                                    }
                                    el.properties
                                        .insert("protection".into(), protections.join(", "));
                                }
                                "branches-from" => {
                                    let target = this.parse_ident()?;
                                    let resolved = this.resolve_ref(&target);
                                    el.properties
                                        .insert("branches-from".into(), resolved.clone());
                                    this.model.add_relationship(Relationship {
                                        frm: resolved,
                                        to: branch_id_for_body.clone(),
                                        label: "branches from".into(),
                                        technology: None,
                                        order: None,
                                    });
                                }
                                "merges-into" => {
                                    let target = this.parse_ident()?;
                                    let resolved = this.resolve_ref(&target);
                                    el.properties.insert("merges-into".into(), resolved.clone());
                                    this.model.add_relationship(Relationship {
                                        frm: branch_id_for_body.clone(),
                                        to: resolved,
                                        label: "merges into".into(),
                                        technology: None,
                                        order: None,
                                    });
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
                    this.model.add_element(el);
                } else {
                    if this.peek_after_ws() == Some('"') {
                        this.parse_string()?;
                    }
                    if this.peek_after_ws() == Some('{') {
                        this.skip_block()?;
                    }
                }
            } else {
                if this.peek_after_ws() == Some('"') {
                    this.parse_string()?;
                }
                if this.peek_after_ws() == Some('{') {
                    this.skip_block()?;
                }
            }
            Ok(())
        })
    }

    fn parse_pipeline(&mut self, pipeline_id: String) -> Result<(), ParseError> {
        let pipeline_name = self.parse_string()?;
        self.model.add_element(Element::new(
            &pipeline_id,
            ElementKind::Pipeline,
            &pipeline_name,
        ));
        self.id_map.insert(pipeline_id.clone(), pipeline_id.clone());

        self.parse_braced("pipeline", |this| {
            let first = this.parse_ident()?;
            this.skip_ws();

            if first == "triggers" {
                // `triggers <repo-ref> "event"`. Tokenised properly rather
                // than the line-scan hack of v1.
                let repo_ref = this.parse_ident()?;
                let _event = this.parse_string()?;
                let resolved = this.resolve_ref(&repo_ref);
                if let Some(el) = this.model.elements.get_mut(&pipeline_id) {
                    el.properties.insert("triggered_by".into(), resolved);
                }
                return Ok(());
            }

            if first == "tags" {
                // Quoted-string list; attach to the pipeline element.
                let mut tags: Vec<String> = Vec::new();
                while this.peek_after_ws() == Some('"') {
                    tags.push(this.parse_string()?);
                }
                if let Some(el) = this.model.elements.get_mut(&pipeline_id) {
                    el.tags.extend(tags);
                }
                return Ok(());
            }

            if this.peek() == Some('=') {
                this.advance();
                let kind_str = this.parse_ident()?;
                if kind_str == "stage" {
                    let stage_name = this.parse_string()?;
                    let stage_id = format!("{}.{}", pipeline_id, first);
                    let mut el = Element::new(&stage_id, ElementKind::Stage, &stage_name);
                    el.parent = Some(pipeline_id.clone());
                    this.id_map.insert(first.clone(), stage_id.clone());

                    if this.peek_after_ws() == Some('{') {
                        let stage_id_for_body = stage_id.clone();
                        this.parse_braced("stage", |this| {
                            let prop = this.parse_ident()?;
                            match prop.as_str() {
                                "needs" => {
                                    let dep = this.parse_ident()?;
                                    let dep_full = this.resolve_ref(&dep);
                                    this.model.stage_links.push(StageLink {
                                        frm: dep_full,
                                        to: stage_id_for_body.clone(),
                                    });
                                }
                                "step" => {
                                    this.parse_string()?;
                                }
                                "environment" => {
                                    let env = this.parse_ident()?;
                                    el.properties.insert("environment".into(), env);
                                }
                                "tags" => {
                                    while this.peek_after_ws() == Some('"') {
                                        el.tags.push(this.parse_string()?);
                                    }
                                }
                                "gate" => {
                                    let gate_name = this.parse_string()?;
                                    let gate_id = format!("{}.gate", stage_id_for_body);
                                    let mut gate =
                                        Element::new(&gate_id, ElementKind::Gate, &gate_name);
                                    gate.parent = Some(stage_id_for_body.clone());
                                    if this.peek_after_ws() == Some('{') {
                                        this.parse_braced("gate", |this| {
                                            let gp = this.parse_ident()?;
                                            if this.peek_after_ws() == Some('"') {
                                                gate.properties.insert(gp, this.parse_string()?);
                                            }
                                            Ok(())
                                        })?;
                                    }
                                    this.model.add_element(gate);
                                }
                                _ => {
                                    return Err(
                                        this.error(format!("unknown stage keyword '{}'", prop))
                                    );
                                }
                            }
                            Ok(())
                        })?;
                    }
                    this.model.add_element(el);
                } else {
                    return Err(this.error(format!(
                        "unknown pipeline element kind '{}', expected 'stage'",
                        kind_str
                    )));
                }
                return Ok(());
            }

            Err(this.error(format!("unknown pipeline keyword '{}'", first)))
        })
    }
}

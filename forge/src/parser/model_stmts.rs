//! `model { ... }` block: element declarations and relationships.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_model(&mut self) -> Result<(), ParseError> {
        self.parse_braced("model", |this| this.parse_model_stmt())
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
                let scope_entry = if self.scope.is_empty() {
                    first.clone()
                } else {
                    full_id.clone()
                };
                self.scope.push(scope_entry);
                let full_id_for_body = full_id.clone();
                self.parse_braced("element", |this| {
                    let saved2 = this.pos;
                    let inner = this.parse_ident()?;
                    this.skip_ws();

                    match inner.as_str() {
                        "description" => {
                            el.description = Some(this.parse_string()?);
                        }
                        "technology" => {
                            el.technology = Some(this.parse_string()?);
                        }
                        "tags" => {
                            while this.peek_after_ws() == Some('"') {
                                el.tags.push(this.parse_string()?);
                            }
                        }
                        "data-class" => {
                            while this.peek_after_ws() == Some('"') {
                                el.data_classes.push(this.parse_string()?);
                            }
                        }
                        _ => {
                            if this.peek() == Some('=') {
                                // Child element
                                this.model.add_element(el.clone());
                                this.pos = saved2;
                                this.parse_model_stmt()?;
                                if let Some(updated) = this.model.elements.get(&full_id_for_body) {
                                    el = updated.clone();
                                }
                            } else if this.peek() == Some('-') {
                                this.pos = saved2;
                                this.parse_relationship()?;
                            } else if this.peek_after_ws() == Some('"') {
                                this.parse_string()?;
                            } else if this.peek_after_ws() == Some('{') {
                                this.skip_block()?;
                            }
                        }
                    }
                    Ok(())
                })?;
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
}

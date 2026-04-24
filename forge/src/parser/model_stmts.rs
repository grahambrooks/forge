//! `model { ... }` block: element declarations and relationships.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_model(&mut self) -> Result<(), ParseError> {
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
}

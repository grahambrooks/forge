//! `deployment { ... }` block: nested deployment nodes with container instances.

use super::{ParseError, Parser};
use crate::model::*;

impl Parser {
    pub(super) fn parse_deployment(&mut self) -> Result<(), ParseError> {
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
}

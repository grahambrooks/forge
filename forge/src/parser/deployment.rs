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
        self.parse_braced("deployment", |this| {
            let kw = this.parse_ident()?;
            if kw == "node" {
                this.parse_deployment_node(&env_id, &env_id)?;
            } else if this.peek_after_ws() == Some('"') {
                this.parse_string()?;
            } else if this.peek_after_ws() == Some('{') {
                this.skip_block()?;
            }
            Ok(())
        })
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

        // First pass: gather properties, then recurse for children
        let node_id_for_body = node_id.clone();
        self.parse_braced("deployment node", |this| {
            let kw = this.parse_ident()?;
            match kw.as_str() {
                "technology" => {
                    el.technology = Some(this.parse_string()?);
                }
                "description" => {
                    el.description = Some(this.parse_string()?);
                }
                "tags" => {
                    while this.peek_after_ws() == Some('"') {
                        el.tags.push(this.parse_string()?);
                    }
                }
                "instances" => {
                    let count = this.parse_ident()?;
                    el.properties.insert("instances".into(), count);
                }
                "node" => {
                    // Save element before recursing so parent exists
                    this.model.add_element(el.clone());
                    this.parse_deployment_node(env_id, &node_id_for_body)?;
                    // Refresh el from model (children may have been added)
                    if let Some(updated) = this.model.elements.get(&node_id_for_body) {
                        el = updated.clone();
                    }
                }
                "instance" => {
                    let container_ref = this.parse_ident()?;
                    let resolved = this.resolve_ref(&container_ref);
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
                    return Err(this.error(format!("unknown deployment-node keyword '{}'", kw)));
                }
            }
            Ok(())
        })?;
        self.model.add_element(el);
        Ok(())
    }
}

//! Parser for .forge-catalog files.
//!
//! Catalog files define multi-project aggregations for enterprise-scale
//! documentation sites. Syntax:
//!
//! ```forge
//! catalog "Enterprise Architecture" {
//!   description "All systems across the organization"
//!
//!   project "payments" {
//!     name "Payment Platform"
//!     description "Card and bank payment processing"
//!     source "./projects/payments/forge.forge"
//!     repository "github.com/acme/payments"
//!     tags "core" "pci"
//!   }
//!
//!   project "catalog-service" {
//!     name "Product Catalog"
//!     source "./projects/catalog/forge.forge"
//!     repository "github.com/acme/catalog"
//!   }
//! }
//! ```

use crate::model::{Catalog, CatalogProject};

pub struct CatalogParser {
    text: Vec<char>,
    pos: usize,
    catalog: Catalog,
}

impl CatalogParser {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            pos: 0,
            catalog: Catalog::default(),
        }
    }

    pub fn parse(mut self) -> Result<Catalog, String> {
        self.skip_ws();
        let kw = self.parse_ident()?;
        if kw != "catalog" {
            return Err(format!("expected 'catalog', got '{}'", kw));
        }
        self.catalog.name = self.parse_string()?;
        self.expect('{')?;

        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err("unexpected EOF".to_string());
            }

            let kw = self.parse_ident()?;
            match kw.as_str() {
                "description" => {
                    self.catalog.description = self.parse_string()?;
                }
                "project" => {
                    let project = self.parse_project()?;
                    self.catalog.projects.push(project);
                }
                _ => return Err(format!("unknown catalog block '{}'", kw)),
            }
        }

        Ok(self.catalog)
    }

    fn parse_project(&mut self) -> Result<CatalogProject, String> {
        let key = self.parse_string()?;
        self.expect('{')?;

        let mut name = key.clone();
        let mut description = None;
        let mut source = String::new();
        let mut repository = None;
        let mut tags = Vec::new();

        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            if self.at_end() {
                return Err("unexpected EOF in project block".to_string());
            }

            let kw = self.parse_ident()?;
            match kw.as_str() {
                "name" => name = self.parse_string()?,
                "description" => description = Some(self.parse_string()?),
                "source" => source = self.parse_string()?,
                "repository" => repository = Some(self.parse_string()?),
                "tags" => {
                    // Parse all subsequent strings as tags until we hit a non-string token
                    loop {
                        self.skip_ws();
                        // Try to parse a string; if it fails or we hit a keyword/brace, stop
                        let start_pos = self.pos;
                        match self.parse_string() {
                            Ok(tag) => {
                                // Check if this looks like a keyword for the next property
                                if tag == "name"
                                    || tag == "description"
                                    || tag == "source"
                                    || tag == "repository"
                                    || tag == "tags"
                                {
                                    // Roll back - this is actually the next property
                                    self.pos = start_pos;
                                    break;
                                }
                                tags.push(tag);
                            }
                            Err(_) => {
                                // Not a string, stop parsing tags
                                self.pos = start_pos;
                                break;
                            }
                        }
                    }
                }
                _ => return Err(format!("unknown project property '{}'", kw)),
            }
        }

        if source.is_empty() {
            return Err(format!("project '{}' missing required 'source' field", key));
        }

        Ok(CatalogProject {
            key,
            name,
            description,
            source,
            repository,
            tags,
            mtime: None,
        })
    }

    // ────────────────────────────────────────────────────────────────────
    // Lexer primitives
    // ────────────────────────────────────────────────────────────────────

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.text.get(self.pos + 1) == Some(&'/') {
                // Line comment
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
            } else if c == '/' && self.text.get(self.pos + 1) == Some(&'*') {
                // Block comment
                self.advance();
                self.advance();
                while !self.at_end() {
                    if self.peek() == Some('*') && self.text.get(self.pos + 1) == Some(&'/') {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.advance();
            Ok(())
        } else {
            Err(format!("expected '{}', got {:?}", c, self.peek()))
        }
    }

    fn parse_ident(&mut self) -> Result<String, String> {
        self.skip_ws();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err("expected identifier".to_string());
        }
        Ok(self.text[start..self.pos].iter().collect())
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.peek() == Some('"') {
            self.advance();
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == '"' {
                    let s: String = self.text[start..self.pos].iter().collect();
                    self.advance();
                    return Ok(s);
                } else if c == '\\' {
                    self.advance();
                    if self.at_end() {
                        return Err("unexpected EOF in string".to_string());
                    }
                    self.advance();
                } else {
                    self.advance();
                }
            }
            Err("unterminated string".to_string())
        } else {
            // Unquoted string (identifier-like)
            self.parse_ident()
        }
    }
}

/// Parse a catalog from text.
pub fn parse_catalog(text: &str) -> Result<Catalog, String> {
    CatalogParser::new(text).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_catalog() {
        let input = r#"catalog "Test" {}"#;
        let result = parse_catalog(input);
        assert!(result.is_ok());
        let catalog = result.unwrap();
        assert_eq!(catalog.name, "Test");
        assert_eq!(catalog.projects.len(), 0);
    }

    #[test]
    fn test_parse_catalog_with_description() {
        let input = r#"
            catalog "Enterprise" {
              description "All systems"
            }
        "#;
        let result = parse_catalog(input);
        assert!(result.is_ok());
        let catalog = result.unwrap();
        assert_eq!(catalog.name, "Enterprise");
        assert_eq!(catalog.description, "All systems");
    }

    #[test]
    fn test_parse_catalog_with_projects() {
        let input = r#"
            catalog "Test Catalog" {
              description "Testing"

              project "payments" {
                name "Payment Platform"
                description "Card payments"
                source "./payments.forge"
                repository "github.com/acme/payments"
                tags "core" "pci"
              }

              project "catalog" {
                name "Product Catalog"
                source "./catalog.forge"
              }
            }
        "#;
        let result = parse_catalog(input);
        if let Err(e) = &result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let catalog = result.unwrap();
        assert_eq!(catalog.projects.len(), 2);

        let p1 = &catalog.projects[0];
        assert_eq!(p1.key, "payments");
        assert_eq!(p1.name, "Payment Platform");
        assert_eq!(p1.description, Some("Card payments".to_string()));
        assert_eq!(p1.source, "./payments.forge");
        assert_eq!(p1.repository, Some("github.com/acme/payments".to_string()));
        assert_eq!(p1.tags.len(), 2);

        let p2 = &catalog.projects[1];
        assert_eq!(p2.key, "catalog");
        assert_eq!(p2.name, "Product Catalog");
        assert!(p2.description.is_none());
    }

    #[test]
    fn test_parse_project_missing_source() {
        let input = r#"
            catalog "Test" {
              project "bad" {
                name "Missing Source"
              }
            }
        "#;
        let result = parse_catalog(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing required 'source'"));
    }

    #[test]
    fn test_parse_catalog_with_comments() {
        let input = r#"
            // This is a test catalog
            catalog "Test" {
              /* Multi-line
                 comment */
              description "Test desc"

              // Project definition
              project "test" {
                name "Test Project"
                source "./test.forge"
              }
            }
        "#;
        let result = parse_catalog(input);
        assert!(result.is_ok());
        let catalog = result.unwrap();
        assert_eq!(catalog.name, "Test");
        assert_eq!(catalog.projects.len(), 1);
    }
}

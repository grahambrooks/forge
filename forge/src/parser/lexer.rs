//! Low-level scanner: position, whitespace/comments, primitive tokens, and
//! scope/id-map helpers shared by every `parse_*` routine.

use super::{ParseError, Parser};

impl Parser {
    pub(super) fn line_col(&self) -> (usize, usize) {
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

    pub(super) fn error(&self, msg: impl Into<String>) -> ParseError {
        let (line, col) = self.line_col();
        ParseError {
            msg: msg.into(),
            line,
            col,
        }
    }

    pub(super) fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    pub(super) fn peek(&self) -> Option<char> {
        self.text.get(self.pos).copied()
    }

    pub(super) fn advance(&mut self) -> Option<char> {
        if self.pos < self.text.len() {
            let c = self.text[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    pub(super) fn skip_ws(&mut self) {
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

    pub(super) fn expect(&mut self, ch: char) -> Result<(), ParseError> {
        self.skip_ws();
        match self.advance() {
            Some(c) if c == ch => Ok(()),
            Some(c) => Err(self.error(format!("expected '{}', got '{}'", ch, c))),
            None => Err(self.error(format!("expected '{}', got EOF", ch))),
        }
    }

    pub(super) fn parse_string(&mut self) -> Result<String, ParseError> {
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

    pub(super) fn parse_ident(&mut self) -> Result<String, ParseError> {
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

    pub(super) fn peek_after_ws(&mut self) -> Option<char> {
        let saved = self.pos;
        self.skip_ws();
        let c = self.peek();
        self.pos = saved;
        c
    }

    pub(super) fn skip_block(&mut self) -> Result<(), ParseError> {
        self.expect('{')?;
        let mut depth = 1;
        while depth > 0 {
            match self.advance() {
                None => return Err(self.error("unexpected EOF in block")),
                Some('{') => depth += 1,
                Some('}') => depth -= 1,
                Some('"') => loop {
                    match self.advance() {
                        None => return Err(self.error("unterminated string")),
                        Some('"') => break,
                        Some('\\') => {
                            self.advance();
                        }
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn scoped_id(&self, local: &str) -> String {
        if self.scope.is_empty() {
            local.to_string()
        } else {
            format!("{}.{}", self.scope.last().unwrap(), local)
        }
    }

    pub(super) fn resolve_ref(&self, name: &str) -> String {
        if let Some(full) = self.id_map.get(name) {
            return full.clone();
        }
        if !self.scope.is_empty() {
            let scoped = format!("{}.{}", self.scope.last().unwrap(), name);
            if self.model.elements.contains_key(&scoped) {
                return scoped;
            }
        }
        name.to_string()
    }

    /// Parse an unsigned integer literal. Used by dynamic view step numbers
    /// and composite view grid dimensions.
    pub(super) fn parse_u32(&mut self) -> Result<u32, ParseError> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.text.len() && self.text[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("expected number"));
        }
        let s: String = self.text[start..self.pos].iter().collect();
        s.parse::<u32>().map_err(|_| self.error("invalid number"))
    }
}

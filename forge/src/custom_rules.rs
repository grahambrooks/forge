//! Custom lint rules — parses .forge-rules files and evaluates them against a model.
//!
//! Rule syntax:
//!   rule "name" {
//!     description "..."
//!     severity error|warning|info
//!     scope container|system|person|component|relationship
//!     condition count(outgoing_relationships) > N
//!     condition technology != ""
//!     message "..."
//!   }

use crate::check::{Severity, Violation};
use crate::model::*;

#[derive(Debug, Clone)]
pub struct CustomRule {
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub scope: String,
    pub conditions: Vec<String>,
    pub message_template: String,
}

/// Parse a .forge-rules file into a list of CustomRule.
pub fn parse_rules(text: &str) -> Result<Vec<CustomRule>, String> {
    let mut rules = Vec::new();
    let mut pos = 0;
    let chars: Vec<char> = text.chars().collect();

    while pos < chars.len() {
        skip_ws_and_comments(&chars, &mut pos);
        if pos >= chars.len() {
            break;
        }

        let word = read_ident(&chars, &mut pos);
        if word == "rule" {
            skip_ws_and_comments(&chars, &mut pos);
            let name = read_string(&chars, &mut pos)?;
            skip_ws_and_comments(&chars, &mut pos);
            expect_char(&chars, &mut pos, '{')?;

            let mut rule = CustomRule {
                name,
                description: String::new(),
                severity: Severity::Warning,
                scope: "container".into(),
                conditions: Vec::new(),
                message_template: String::new(),
            };

            loop {
                skip_ws_and_comments(&chars, &mut pos);
                if pos < chars.len() && chars[pos] == '}' {
                    pos += 1;
                    break;
                }
                if pos >= chars.len() {
                    return Err("unterminated rule block".into());
                }
                let prop = read_ident(&chars, &mut pos);
                skip_ws_and_comments(&chars, &mut pos);
                match prop.as_str() {
                    "description" => rule.description = read_string(&chars, &mut pos)?,
                    "severity" => {
                        let s = read_ident(&chars, &mut pos);
                        rule.severity = Severity::from_str(&s)
                            .ok_or_else(|| format!("invalid severity '{}'", s))?;
                    }
                    "scope" => rule.scope = read_ident(&chars, &mut pos),
                    "condition" => {
                        let cond = read_to_eol(&chars, &mut pos);
                        rule.conditions.push(cond);
                    }
                    "message" => rule.message_template = read_string(&chars, &mut pos)?,
                    _ => {
                        // Skip unknown
                        if pos < chars.len() && chars[pos] == '"' {
                            read_string(&chars, &mut pos)?;
                        } else {
                            read_to_eol(&chars, &mut pos);
                        }
                    }
                }
            }
            rules.push(rule);
        }
    }

    Ok(rules)
}

/// Evaluate custom rules against a model, returning violations.
pub fn evaluate_rules(rules: &[CustomRule], model: &Model) -> Vec<Violation> {
    let mut violations = Vec::new();

    for rule in rules {
        match rule.scope.as_str() {
            "container" | "system" | "person" | "component" => {
                let target_kind = match rule.scope.as_str() {
                    "container" => Some(ElementKind::Container),
                    "system" => Some(ElementKind::System),
                    "person" => Some(ElementKind::Person),
                    "component" => Some(ElementKind::Component),
                    _ => None,
                };
                for el in model.elements.values() {
                    if let Some(tk) = target_kind {
                        if el.kind != tk {
                            continue;
                        }
                    }
                    if check_conditions(&rule.conditions, el, model) {
                        let msg = render_message(&rule.message_template, el, model);
                        violations.push(Violation {
                            rule: leak_string(&rule.name),
                            severity: rule.severity,
                            message: msg,
                            element_id: Some(el.id.clone()),
                        });
                    }
                }
            }
            "relationship" => {
                for rel in &model.relationships {
                    if check_rel_conditions(&rule.conditions, rel, model) {
                        let msg = rule
                            .message_template
                            .replace("{source.name}", &rel.frm)
                            .replace("{target.name}", &rel.to)
                            .replace("{technology}", rel.technology.as_deref().unwrap_or(""));
                        violations.push(Violation {
                            rule: leak_string(&rule.name),
                            severity: rule.severity,
                            message: msg,
                            element_id: Some(rel.frm.clone()),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    violations
}

// ─── Condition evaluation ────────────────────────────────────────

fn check_conditions(conditions: &[String], el: &Element, model: &Model) -> bool {
    conditions.iter().all(|c| eval_condition(c, el, model))
}

fn eval_condition(cond: &str, el: &Element, model: &Model) -> bool {
    let cond = cond.trim();

    // count(outgoing_relationships) > N
    if let Some(rest) = cond.strip_prefix("count(outgoing_relationships)") {
        let count = model
            .relationships
            .iter()
            .filter(|r| r.frm == el.id)
            .count();
        return eval_numeric_comparison(rest.trim(), count);
    }

    // count(incoming_relationships) > N
    if let Some(rest) = cond.strip_prefix("count(incoming_relationships)") {
        let count = model.relationships.iter().filter(|r| r.to == el.id).count();
        return eval_numeric_comparison(rest.trim(), count);
    }

    // technology == "value" or technology != ""
    if let Some(rest) = cond.strip_prefix("technology") {
        let tech = el.technology.as_deref().unwrap_or("");
        return eval_string_comparison(rest.trim(), tech);
    }

    // description == "" (missing description)
    if let Some(rest) = cond.strip_prefix("description") {
        let desc = el.description.as_deref().unwrap_or("");
        return eval_string_comparison(rest.trim(), desc);
    }

    // has_tag "value"
    if let Some(rest) = cond.strip_prefix("has_tag") {
        let tag = rest.trim().trim_matches('"');
        return el.tags.contains(&tag.to_string());
    }

    // not has_tag "value"
    if let Some(rest) = cond.strip_prefix("not has_tag") {
        let tag = rest.trim().trim_matches('"');
        return !el.tags.contains(&tag.to_string());
    }

    false
}

fn eval_numeric_comparison(expr: &str, value: usize) -> bool {
    let expr = expr.trim();
    if let Some(n) = expr.strip_prefix('>') {
        n.trim().parse::<usize>().is_ok_and(|n| value > n)
    } else if let Some(n) = expr.strip_prefix('<') {
        n.trim().parse::<usize>().is_ok_and(|n| value < n)
    } else if let Some(n) = expr.strip_prefix("==") {
        n.trim().parse::<usize>() == Ok(value)
    } else {
        false
    }
}

fn eval_string_comparison(expr: &str, value: &str) -> bool {
    let expr = expr.trim();
    if let Some(rest) = expr.strip_prefix("!=") {
        let expected = rest.trim().trim_matches('"');
        value != expected
    } else if let Some(rest) = expr.strip_prefix("==") {
        let expected = rest.trim().trim_matches('"');
        value == expected
    } else {
        false
    }
}

fn check_rel_conditions(conditions: &[String], rel: &Relationship, model: &Model) -> bool {
    conditions.iter().all(|c| {
        let c = c.trim();
        if let Some(rest) = c.strip_prefix("source.kind") {
            if let Some(source) = model.elements.get(&rel.frm) {
                let kind_str = format!("{:?}", source.kind).to_lowercase();
                return eval_string_comparison(rest.trim(), &kind_str);
            }
        }
        if let Some(rest) = c.strip_prefix("technology") {
            let tech = rel.technology.as_deref().unwrap_or("");
            return eval_string_comparison(rest.trim(), tech);
        }
        false
    })
}

fn render_message(template: &str, el: &Element, model: &Model) -> String {
    let outgoing = model
        .relationships
        .iter()
        .filter(|r| r.frm == el.id)
        .count();
    template
        .replace("{element.name}", &el.name)
        .replace("{element.id}", &el.id)
        .replace("{count}", &outgoing.to_string())
        .replace("{technology}", el.technology.as_deref().unwrap_or("none"))
}

// ─── Parser helpers ──────────────────────────────────────────────

fn skip_ws_and_comments(chars: &[char], pos: &mut usize) {
    loop {
        while *pos < chars.len() && chars[*pos].is_whitespace() {
            *pos += 1;
        }
        if *pos + 1 < chars.len() && chars[*pos] == '/' && chars[*pos + 1] == '/' {
            while *pos < chars.len() && chars[*pos] != '\n' {
                *pos += 1;
            }
            continue;
        }
        break;
    }
}

fn read_ident(chars: &[char], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < chars.len()
        && (chars[*pos].is_alphanumeric()
            || chars[*pos] == '_'
            || chars[*pos] == '-'
            || chars[*pos] == '.')
    {
        *pos += 1;
    }
    chars[start..*pos].iter().collect()
}

fn read_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    skip_ws_and_comments(chars, pos);
    if *pos >= chars.len() || chars[*pos] != '"' {
        return Err("expected quoted string".into());
    }
    *pos += 1;
    let mut s = String::new();
    while *pos < chars.len() {
        if chars[*pos] == '"' {
            *pos += 1;
            return Ok(s);
        }
        if chars[*pos] == '\\' && *pos + 1 < chars.len() {
            *pos += 1;
        }
        s.push(chars[*pos]);
        *pos += 1;
    }
    Err("unterminated string".into())
}

fn read_to_eol(chars: &[char], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < chars.len() && chars[*pos] != '\n' {
        *pos += 1;
    }
    chars[start..*pos]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

fn expect_char(chars: &[char], pos: &mut usize, ch: char) -> Result<(), String> {
    skip_ws_and_comments(chars, pos);
    if *pos < chars.len() && chars[*pos] == ch {
        *pos += 1;
        Ok(())
    } else {
        Err(format!("expected '{}'", ch))
    }
}

/// Leak a string to get a &'static str for Violation.rule.
/// This is fine for a small number of custom rules.
fn leak_string(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_rule() {
        let text = r#"
rule "max-coupling" {
    description "No container should have more than 5 outgoing deps"
    severity error
    scope container
    condition count(outgoing_relationships) > 5
    message "{element.name} has too many outgoing dependencies ({count})"
}
"#;
        let rules = parse_rules(text).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "max-coupling");
        assert_eq!(rules[0].severity, Severity::Error);
        assert_eq!(rules[0].scope, "container");
        assert_eq!(rules[0].conditions.len(), 1);
    }

    #[test]
    fn parse_multiple_rules() {
        let text = r#"
rule "require-tech" {
    severity warning
    scope container
    condition technology == ""
    message "{element.name} has no technology"
}
rule "require-https" {
    severity error
    scope relationship
    condition source.kind == "person"
    condition technology != "HTTPS"
    message "External relationship uses {technology}, expected HTTPS"
}
"#;
        let rules = parse_rules(text).unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[1].conditions.len(), 2);
    }

    #[test]
    fn evaluate_coupling_rule() {
        let rules_text = r#"
rule "max-deps" {
    severity error
    scope container
    condition count(outgoing_relationships) > 2
    message "{element.name} has too many deps"
}
"#;
        let rules = parse_rules(rules_text).unwrap();
        let mut model = Model::default();
        model.add_element(Element::new("svc", ElementKind::Container, "Service"));
        model.add_element(Element::new("a", ElementKind::Container, "A"));
        model.add_element(Element::new("b", ElementKind::Container, "B"));
        model.add_element(Element::new("c", ElementKind::Container, "C"));
        for target in ["a", "b", "c"] {
            model.add_relationship(Relationship {
                frm: "svc".into(),
                to: target.into(),
                label: "uses".into(),
                technology: None,
            });
        }

        let violations = evaluate_rules(&rules, &model);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Service"));
    }

    #[test]
    fn condition_no_match_no_violation() {
        let rules_text = r#"
rule "test" {
    severity warning
    scope container
    condition count(outgoing_relationships) > 10
    message "too many"
}
"#;
        let rules = parse_rules(rules_text).unwrap();
        let mut model = Model::default();
        model.add_element(Element::new("svc", ElementKind::Container, "Svc"));

        let violations = evaluate_rules(&rules, &model);
        assert!(violations.is_empty());
    }
}

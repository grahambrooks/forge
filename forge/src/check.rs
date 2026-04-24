//! Forge architectural linter — built-in rules for model validation.
//!
//! Each rule lives in [`rules`]; this file owns the shared [`Violation`] and
//! [`Severity`] types and the dispatcher.

use crate::model::*;

mod rules;

#[cfg(test)]
mod tests;

use rules::{
    check_chatty_coupling, check_data_class_boundary, check_database_direct_access,
    check_dependency_cycles, check_empty_views, check_gate_coverage, check_missing_descriptions,
    check_missing_technology, check_orphaned_elements,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

impl Severity {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Severity::Info),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
    pub element_id: Option<String>,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.element_id.as_deref().unwrap_or("-");
        write!(
            f,
            "[{}] {} ({}): {}",
            self.severity, self.rule, id, self.message
        )
    }
}

pub fn check(model: &Model, min_severity: Severity) -> Vec<Violation> {
    let mut violations = Vec::new();

    check_missing_descriptions(model, &mut violations);
    check_missing_technology(model, &mut violations);
    check_orphaned_elements(model, &mut violations);
    check_dependency_cycles(model, &mut violations);
    check_database_direct_access(model, &mut violations);
    check_chatty_coupling(model, &mut violations);
    check_gate_coverage(model, &mut violations);
    check_empty_views(model, &mut violations);
    check_data_class_boundary(model, &mut violations);

    violations.retain(|v| v.severity >= min_severity);
    violations.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.rule.cmp(b.rule)));
    violations
}

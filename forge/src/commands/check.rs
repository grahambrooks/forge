//! `forge check` — lint a model against architectural rules. Output formats:
//! text, JSON, SARIF.

use std::fs;
use std::path::Path;
use std::process;

use crate::{check, custom_rules};

use super::util::{die, load_model};

pub fn cmd_check(source: &Path, severity: &str, format: &str, rules: Option<&Path>) {
    let min_severity = check::Severity::from_str(severity)
        .unwrap_or_else(|| die("--severity must be 'error', 'warning', or 'info'"));

    let model = load_model(source);
    let mut violations = check::check(&model, min_severity);

    if let Some(rules_path) = rules {
        let rules_text = fs::read_to_string(rules_path)
            .unwrap_or_else(|e| die(&format!("reading rules: {}: {}", rules_path.display(), e)));
        let custom = custom_rules::parse_rules(&rules_text)
            .unwrap_or_else(|e| die(&format!("parsing rules: {}", e)));
        let mut cv = custom_rules::evaluate_rules(&custom, &model);
        cv.retain(|v| v.severity >= min_severity);
        violations.extend(cv);
    }

    match format {
        "json" => print_json(&violations),
        "sarif" => print_sarif(&violations, source),
        _ => print_text(&violations),
    }

    if violations
        .iter()
        .any(|v| v.severity == check::Severity::Error)
    {
        process::exit(1);
    }
}

fn print_text(violations: &[check::Violation]) {
    if violations.is_empty() {
        eprintln!("No issues found.");
        return;
    }
    eprintln!("Found {} issue(s):\n", violations.len());
    for v in violations {
        println!("{}", v);
    }
}

fn print_json(violations: &[check::Violation]) {
    let results: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "rule": v.rule,
                "severity": v.severity.to_string(),
                "element": v.element_id,
                "message": v.message,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&results).unwrap_or_default()
    );
}

fn print_sarif(violations: &[check::Violation], source: &Path) {
    use serde_json::json;
    let results: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            let level = match v.severity {
                check::Severity::Error => "error",
                check::Severity::Warning => "warning",
                check::Severity::Info => "note",
            };
            json!({
                "ruleId": v.rule,
                "level": level,
                "message": { "text": v.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": source.display().to_string() },
                        "region": { "startLine": 1 }
                    },
                    "logicalLocations": v.element_id.as_ref().map(|id| vec![json!({
                        "name": id, "kind": "element"
                    })]).unwrap_or_default(),
                }]
            })
        })
        .collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "forge",
                    "version": env!("FORGE_VERSION"),
                    "rules": violations.iter().map(|v| json!({
                        "id": v.rule, "shortDescription": { "text": v.rule },
                    })).collect::<Vec<_>>(),
                }
            },
            "results": results,
        }]
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&sarif).unwrap_or_default()
    );
}

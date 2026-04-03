//! Forge preprocessor — resolves directives before parsing.
//!
//! Directives:
//! - `!include <path>` or `!include <glob>` — inline another .forge file
//! - `!fragment "name" { ... }` — define a reusable named block
//! - `!use "name"` — expand a previously defined fragment
//! - `!if env("VAR") == "value" { ... }` — conditional inclusion

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Preprocess a .forge file, resolving all directives.
/// `base_dir` is the directory containing the source file.
pub fn preprocess(text: &str, base_dir: &Path) -> Result<String, PreprocessError> {
    let mut ctx = PreprocessContext {
        fragments: HashMap::new(),
        include_stack: Vec::new(),
    };
    ctx.include_stack.push(base_dir.to_path_buf());
    process_text(text, base_dir, &mut ctx)
}

#[derive(Debug)]
pub struct PreprocessError {
    pub msg: String,
}

impl std::fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Preprocess error: {}", self.msg)
    }
}

impl std::error::Error for PreprocessError {}

struct PreprocessContext {
    fragments: HashMap<String, String>,
    include_stack: Vec<PathBuf>,
}

fn process_text(
    text: &str,
    base_dir: &Path,
    ctx: &mut PreprocessContext,
) -> Result<String, PreprocessError> {
    let mut output = String::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("!include ") {
            let path_str = rest.trim();
            resolve_include(&mut output, path_str, base_dir, ctx)?;
        } else if trimmed.starts_with("!fragment ") {
            let body = extract_fragment_def(trimmed, &mut lines)?;
            let name = extract_quoted_string(trimmed.strip_prefix("!fragment ").unwrap())
                .ok_or_else(|| PreprocessError {
                    msg: "!fragment requires a quoted name".into(),
                })?;
            ctx.fragments.insert(name, body);
        } else if let Some(rest) = trimmed.strip_prefix("!use ") {
            let name = extract_quoted_string(rest).ok_or_else(|| PreprocessError {
                msg: "!use requires a quoted name".into(),
            })?;
            let body = ctx.fragments.get(&name).ok_or_else(|| PreprocessError {
                msg: format!("unknown fragment '{}'", name),
            })?;
            output.push_str(body);
            output.push('\n');
        } else if trimmed.starts_with("!if ") {
            let condition = trimmed.strip_prefix("!if ").unwrap().trim();
            let body = extract_braced_block(condition, &mut lines)?;
            let cond_expr = condition.trim_end_matches('{').trim();
            if evaluate_condition(cond_expr) {
                let processed = process_text(&body, base_dir, ctx)?;
                output.push_str(&processed);
                output.push('\n');
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}

fn resolve_include(
    output: &mut String,
    path_str: &str,
    base_dir: &Path,
    ctx: &mut PreprocessContext,
) -> Result<(), PreprocessError> {
    // Check for glob patterns
    if path_str.contains('*') {
        let pattern = base_dir.join(path_str);
        let pattern_str = pattern.to_string_lossy();
        let mut paths: Vec<PathBuf> = glob_paths(&pattern_str);
        paths.sort(); // deterministic ordering
        if paths.is_empty() {
            return Err(PreprocessError {
                msg: format!("no files matched include pattern '{}'", path_str),
            });
        }
        for path in paths {
            include_file(output, &path, ctx)?;
        }
    } else {
        let path = base_dir.join(path_str);
        include_file(output, &path, ctx)?;
    }
    Ok(())
}

fn include_file(
    output: &mut String,
    path: &Path,
    ctx: &mut PreprocessContext,
) -> Result<(), PreprocessError> {
    let canonical = path.canonicalize().map_err(|e| PreprocessError {
        msg: format!("cannot resolve '{}': {}", path.display(), e),
    })?;

    // Circular include detection
    if ctx.include_stack.contains(&canonical) {
        let chain: Vec<String> = ctx
            .include_stack
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        return Err(PreprocessError {
            msg: format!(
                "circular include detected: {} → {}",
                chain.join(" → "),
                canonical.display()
            ),
        });
    }

    let text = std::fs::read_to_string(&canonical).map_err(|e| PreprocessError {
        msg: format!("cannot read '{}': {}", path.display(), e),
    })?;

    let include_dir = canonical.parent().unwrap_or(path);
    ctx.include_stack.push(canonical.clone());
    let processed = process_text(&text, include_dir, ctx)?;
    ctx.include_stack.pop();

    output.push_str(&format!("// included from {}\n", path.display()));
    output.push_str(&processed);
    output.push('\n');
    Ok(())
}

/// Simple glob matching — supports `*` in the final path component.
fn glob_paths(pattern: &str) -> Vec<PathBuf> {
    let path = Path::new(pattern);
    let dir = path.parent().unwrap_or(Path::new("."));
    let file_pattern = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            glob_match(&file_pattern, &name)
        })
        .map(|e| e.path())
        .collect()
}

/// Simple glob match: `*` matches any sequence of chars, rest is literal.
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return name.starts_with(prefix)
            && name.ends_with(suffix)
            && name.len() >= prefix.len() + suffix.len();
    }
    pattern == name
}

/// Extract the body of a `!fragment "name" { ... }` block.
fn extract_fragment_def<'a, I: Iterator<Item = &'a str>>(
    first_line: &str,
    lines: &mut std::iter::Peekable<I>,
) -> Result<String, PreprocessError> {
    // The opening brace may be on the first line or the next
    let has_open = first_line.contains('{');
    if !has_open {
        // Skip to the line with {
        for line in lines.by_ref() {
            if line.contains('{') {
                break;
            }
        }
    }
    collect_braced_body(lines)
}

/// Extract the body of a `!if ... { ... }` block.
fn extract_braced_block<'a, I: Iterator<Item = &'a str>>(
    first_part: &str,
    lines: &mut std::iter::Peekable<I>,
) -> Result<String, PreprocessError> {
    let has_open = first_part.contains('{');
    if !has_open {
        for line in lines.by_ref() {
            if line.contains('{') {
                break;
            }
        }
    }
    collect_braced_body(lines)
}

fn collect_braced_body<'a, I: Iterator<Item = &'a str>>(
    lines: &mut I,
) -> Result<String, PreprocessError> {
    let mut body = String::new();
    let mut depth = 1;
    for line in lines {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if depth <= 0 {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    if depth > 0 {
        return Err(PreprocessError {
            msg: "unterminated block in directive".into(),
        });
    }
    Ok(body)
}

fn extract_quoted_string(s: &str) -> Option<String> {
    let s = s.trim();
    if let Some(stripped) = s.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        None
    }
}

/// Evaluate a simple condition: `env("VAR") == "value"` or `env("VAR")` (truthy).
fn evaluate_condition(expr: &str) -> bool {
    let expr = expr.trim();

    // env("VAR") == "value"
    if let Some((lhs, rhs)) = expr.split_once("==") {
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        let lhs_val = eval_expr(lhs);
        let rhs_val = eval_expr(rhs);
        return lhs_val == rhs_val;
    }

    // env("VAR") != "value"
    if let Some((lhs, rhs)) = expr.split_once("!=") {
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        let lhs_val = eval_expr(lhs);
        let rhs_val = eval_expr(rhs);
        return lhs_val != rhs_val;
    }

    // env("VAR") — truthy check (non-empty)
    let val = eval_expr(expr);
    !val.is_empty()
}

fn eval_expr(expr: &str) -> String {
    let expr = expr.trim();
    // env("VAR")
    if let Some(inner) = expr.strip_prefix("env(") {
        let inner = inner.trim_end_matches(')');
        let var_name = inner.trim().trim_matches('"');
        return std::env::var(var_name).unwrap_or_default();
    }
    // String literal
    if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
        return expr[1..expr.len() - 1].to_string();
    }
    expr.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn no_directives_passes_through() {
        let input = "forge \"Test\" {\n  model {}\n}\n";
        let result = preprocess(input, Path::new(".")).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn fragment_and_use() {
        let input = r#"
!fragment "db-tags" {
  tags "database"
}
forge "Test" {
  model {
    db = container "DB" {
      !use "db-tags"
    }
  }
}
"#;
        let result = preprocess(input, Path::new(".")).unwrap();
        assert!(result.contains("tags \"database\""));
        assert!(!result.contains("!fragment"));
        assert!(!result.contains("!use"));
    }

    #[test]
    fn if_true_includes_block() {
        env::set_var("FORGE_TEST_ENV", "production");
        let input = r#"
!if env("FORGE_TEST_ENV") == "production" {
  included_content
}
rest
"#;
        let result = preprocess(input, Path::new(".")).unwrap();
        assert!(result.contains("included_content"));
        assert!(result.contains("rest"));
        env::remove_var("FORGE_TEST_ENV");
    }

    #[test]
    fn if_false_excludes_block() {
        env::remove_var("FORGE_TEST_MISSING");
        let input = r#"
!if env("FORGE_TEST_MISSING") == "yes" {
  should_not_appear
}
rest
"#;
        let result = preprocess(input, Path::new(".")).unwrap();
        assert!(!result.contains("should_not_appear"));
        assert!(result.contains("rest"));
    }

    #[test]
    fn if_truthy_env_var() {
        env::set_var("FORGE_TEST_TRUTHY", "1");
        let input = "!if env(\"FORGE_TEST_TRUTHY\") {\n  visible\n}\n";
        let result = preprocess(input, Path::new(".")).unwrap();
        assert!(result.contains("visible"));
        env::remove_var("FORGE_TEST_TRUTHY");
    }

    #[test]
    fn glob_match_works() {
        assert!(glob_match("*.forge", "payments.forge"));
        assert!(glob_match("ci-*", "ci-deploy.yml"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("test.forge", "test.forge"));
        assert!(!glob_match("*.forge", "readme.md"));
    }

    #[test]
    fn evaluate_condition_eq() {
        env::set_var("FORGE_COND_TEST", "prod");
        assert!(evaluate_condition(r#"env("FORGE_COND_TEST") == "prod""#));
        assert!(!evaluate_condition(
            r#"env("FORGE_COND_TEST") == "staging""#
        ));
        assert!(evaluate_condition(r#"env("FORGE_COND_TEST") != "staging""#));
        env::remove_var("FORGE_COND_TEST");
    }

    #[test]
    fn unknown_fragment_errors() {
        let input = "!use \"nonexistent\"\n";
        let result = preprocess(input, Path::new("."));
        assert!(result.is_err());
        assert!(result.unwrap_err().msg.contains("unknown fragment"));
    }

    #[test]
    fn include_file_resolves() {
        // Test with an actual file
        let text = include_str!("../examples/payments.forge");
        // Should pass through without error (no directives in payments.forge currently)
        let result = preprocess(text, Path::new("examples")).unwrap();
        assert!(result.contains("forge \"Payment Platform\""));
    }
}

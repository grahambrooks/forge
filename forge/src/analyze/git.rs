//! Git scanner — infers branching strategy, active branches, and contributor
//! ownership from git history using gix.

use std::collections::HashMap;
use std::path::Path;

use crate::model::*;

use super::provenance::mark_inferred;
use super::AnalyzeConfig;

const SCANNER: &str = "git";

pub fn scan(model: &mut Model, root: &Path, _config: &AnalyzeConfig) {
    // CODEOWNERS doesn't require a git repo to be useful — parse it first so
    // team attribution works even in non-repo scan targets (e.g. fixtures).
    let codeowners_matched = scan_codeowners(model, root);

    let repo = match gix::open(root) {
        Ok(r) => r,
        Err(_) => return, // not a git repo, skip the rest silently
    };

    scan_branches(model, &repo);
    // Only fall back to contributor-derived teams when CODEOWNERS didn't
    // assign anyone. A real CODEOWNERS file is strictly better signal than
    // "who committed most recently".
    if !codeowners_matched {
        scan_contributors(model, &repo, root);
    }
}

fn scan_branches(model: &mut Model, repo: &gix::Repository) {
    let refs = match repo.references() {
        Ok(r) => r,
        Err(_) => return,
    };

    let local_branches = match refs.local_branches() {
        Ok(b) => b,
        Err(_) => return,
    };

    let mut branch_names: Vec<String> = Vec::new();
    for reference in local_branches.flatten() {
        let name_str = reference.name().as_bstr().to_string();
        let short: String = name_str
            .strip_prefix("refs/heads/")
            .unwrap_or(&name_str)
            .to_string();
        branch_names.push(short);
    }

    if branch_names.is_empty() {
        return;
    }

    // Infer branching strategy
    let has_main = branch_names.iter().any(|b| b == "main" || b == "master");
    let has_develop = branch_names.iter().any(|b| b == "develop" || b == "dev");
    let has_release = branch_names.iter().any(|b| b.starts_with("release/"));
    let has_feature = branch_names.iter().any(|b| b.starts_with("feature/"));
    let has_hotfix = branch_names.iter().any(|b| b.starts_with("hotfix/"));

    let strategy_name = if has_develop && (has_release || has_hotfix) {
        "git-flow"
    } else if has_main && has_feature && !has_develop {
        "trunk-based"
    } else if has_main {
        "github-flow"
    } else {
        "unknown"
    };

    let strategy_id = strategy_name.to_string();

    // Create trunk branch
    let trunk_name = if branch_names.contains(&"main".to_string()) {
        "main"
    } else if branch_names.contains(&"master".to_string()) {
        "master"
    } else {
        branch_names.first().map(|s| s.as_str()).unwrap_or("main")
    };

    let trunk_id = format!("{}.trunk", strategy_id);
    let mut trunk = Element::new(&trunk_id, ElementKind::Branch, trunk_name);
    trunk
        .properties
        .insert("strategy".into(), strategy_id.clone());
    mark_inferred(&mut trunk, SCANNER, None);
    trunk.tags.push("trunk".into());
    model.add_element(trunk);

    // Create feature branch pattern if detected
    if has_feature {
        let feat_id = format!("{}.feature", strategy_id);
        let mut feat = Element::new(&feat_id, ElementKind::Branch, "feature/*");
        feat.properties
            .insert("strategy".into(), strategy_id.clone());
        feat.properties
            .insert("branches-from".into(), trunk_id.clone());
        feat.properties
            .insert("merges-into".into(), trunk_id.clone());
        mark_inferred(&mut feat, SCANNER, None);
        model.add_element(feat);
        model.add_relationship(Relationship {
            frm: trunk_id.clone(),
            to: feat_id.clone(),
            label: "branches from".into(),
            technology: None,
            order: None,
        });
        model.add_relationship(Relationship {
            frm: feat_id,
            to: trunk_id.clone(),
            label: "merges into".into(),
            technology: None,
            order: None,
        });
    }

    // Create develop branch for git-flow
    if has_develop {
        let dev_name = if branch_names.contains(&"develop".to_string()) {
            "develop"
        } else {
            "dev"
        };
        let dev_id = format!("{}.develop", strategy_id);
        let mut dev = Element::new(&dev_id, ElementKind::Branch, dev_name);
        dev.properties
            .insert("strategy".into(), strategy_id.clone());
        dev.properties
            .insert("branches-from".into(), trunk_id.clone());
        mark_inferred(&mut dev, SCANNER, None);
        model.add_element(dev);
    }

    // Create release branch pattern for git-flow
    if has_release {
        let rel_id = format!("{}.release", strategy_id);
        let mut rel = Element::new(&rel_id, ElementKind::Branch, "release/*");
        rel.properties
            .insert("strategy".into(), strategy_id.clone());
        mark_inferred(&mut rel, SCANNER, None);
        model.add_element(rel);
    }
}

// ── CODEOWNERS parsing ─────────────────────────────────────────────

/// Parse a CODEOWNERS file and attribute existing containers to teams based
/// on the rules. Returns true when at least one container was matched, so the
/// caller can skip the commit-history fallback.
///
/// CODEOWNERS locations checked (GitHub-compatible order): `.github/CODEOWNERS`,
/// `CODEOWNERS` at the repo root, `docs/CODEOWNERS`. First file found wins.
///
/// Matching rules follow the CODEOWNERS spec: the *last* matching rule for a
/// given path takes precedence (later rules override earlier ones).
fn scan_codeowners(model: &mut Model, root: &Path) -> bool {
    let candidates = [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"];
    let text = candidates
        .iter()
        .map(|p| root.join(p))
        .find_map(|p| std::fs::read_to_string(p).ok());
    let text = match text {
        Some(t) => t,
        None => return false,
    };

    let rules = parse_codeowners(&text);
    if rules.is_empty() {
        return false;
    }

    // Build (container_id, source_path) pairs so we can run the matcher
    // without holding a borrow on `model.elements`.
    let sources: Vec<(String, String)> = model
        .elements
        .iter()
        .filter(|(_, el)| el.kind == ElementKind::Container)
        .filter_map(|(id, el)| {
            el.properties
                .get("forge:source")
                .map(|s| (id.clone(), s.clone()))
        })
        .collect();

    // team name → owned container ids
    let mut team_owns: HashMap<String, Vec<String>> = HashMap::new();
    let mut any_matched = false;

    for (container_id, source_path) in &sources {
        if let Some(rule) = last_matching_rule(&rules, source_path) {
            for owner in &rule.owners {
                let team_name = normalise_owner(owner);
                let entry = team_owns.entry(team_name).or_default();
                if !entry.contains(container_id) {
                    entry.push(container_id.clone());
                    any_matched = true;
                }
            }
        }
    }

    for (name, owns) in team_owns {
        // Dedupe against any team with the same name (e.g. an earlier
        // fallback contributor entry).
        if let Some(existing) = model.teams.iter_mut().find(|t| t.name == name) {
            for o in owns {
                if !existing.owns.contains(&o) {
                    existing.owns.push(o);
                }
            }
        } else {
            model.teams.push(Team {
                name,
                owns,
                contact: None,
            });
        }
    }

    any_matched
}

#[derive(Debug, Clone)]
struct CodeownersRule {
    pattern: String,
    owners: Vec<String>,
}

fn parse_codeowners(text: &str) -> Vec<CodeownersRule> {
    let mut rules = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Strip inline comments (# preceded by whitespace).
        let line = match line.find(" #") {
            Some(i) => line[..i].trim(),
            None => line,
        };
        let mut parts = line.split_whitespace();
        let pattern = match parts.next() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let owners: Vec<String> = parts.map(String::from).collect();
        if owners.is_empty() {
            // A pattern with no owners explicitly clears ownership for
            // matching paths — record it so later-matching rules still win.
            rules.push(CodeownersRule {
                pattern,
                owners: Vec::new(),
            });
            continue;
        }
        rules.push(CodeownersRule { pattern, owners });
    }
    rules
}

/// Return the last rule in the file that matches `path` (CODEOWNERS spec:
/// later rules override earlier ones).
fn last_matching_rule<'a>(rules: &'a [CodeownersRule], path: &str) -> Option<&'a CodeownersRule> {
    rules
        .iter()
        .rev()
        .find(|r| codeowners_match(&r.pattern, path))
        .filter(|r| !r.owners.is_empty())
}

/// Minimal CODEOWNERS pattern matcher. Supports:
///   * literal directory prefix (`crates/api`)
///   * leading `/` anchoring at repo root (`/src/` → only top-level `src/`)
///   * trailing `/` meaning "directory"
///   * `*` at the end (`crates/*` → immediate subdirs of `crates/`)
///   * `**` for recursive globbing
///   * extension patterns (`*.rs`)
///
/// This isn't a full gitignore implementation but covers the patterns that
/// appear in ~99% of real CODEOWNERS files.
fn codeowners_match(pattern: &str, path: &str) -> bool {
    let pat = pattern.trim_start_matches('/');
    let path = path.trim_start_matches('/');

    // `*` alone: matches everything.
    if pat == "*" || pat.is_empty() {
        return true;
    }

    // Extension patterns: `*.rs`, `*.md`, etc.
    if let Some(ext) = pat.strip_prefix("*.") {
        return path.rsplit('.').next() == Some(ext);
    }

    // Recursive glob: `foo/**` matches any path whose first components are
    // exactly `foo`, or `**/foo` matches any path ending in `foo`.
    if let Some(prefix) = pat.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(suffix) = pat.strip_prefix("**/") {
        return path == suffix
            || path.ends_with(&format!("/{suffix}"))
            || path.starts_with(&format!("{suffix}/"));
    }

    // Single-segment wildcard: `crates/*` → immediate children of crates/.
    if let Some(prefix) = pat.strip_suffix("/*") {
        let remainder = match path.strip_prefix(&format!("{prefix}/")) {
            Some(r) => r,
            None => return false,
        };
        // Exactly one more path segment beyond the prefix.
        return !remainder.is_empty() && !remainder.contains('/');
    }

    // Directory match: `crates/api/` — matches anything under that dir.
    if let Some(dir) = pat.strip_suffix('/') {
        return path == dir || path.starts_with(&format!("{dir}/"));
    }

    // Literal path or directory prefix: `crates/api` matches the file or any
    // descendant path (GitHub treats this as a directory prefix).
    path == pat || path.starts_with(&format!("{pat}/"))
}

/// `@org/platform` → `platform`; `@alice` → `alice`. Email literals are kept
/// as-is.
fn normalise_owner(owner: &str) -> String {
    let trimmed = owner.trim_start_matches('@');
    if let Some(slash) = trimmed.rfind('/') {
        trimmed[slash + 1..].to_string()
    } else {
        trimmed.to_string()
    }
}

fn scan_contributors(model: &mut Model, repo: &gix::Repository, _root: &Path) {
    // Walk recent commits to find contributor → path mappings
    let head = match repo.head_commit() {
        Ok(h) => h,
        Err(_) => return,
    };

    let mut author_commits: HashMap<String, usize> = HashMap::new();

    // Walk recent commits to find contributors
    let mut current_id = head.id;
    for _ in 0..200 {
        let commit = match repo.find_object(current_id) {
            Ok(obj) => match obj.try_into_commit() {
                Ok(c) => c,
                Err(_) => break,
            },
            Err(_) => break,
        };

        let author = commit
            .author()
            .map(|a| a.name.to_string())
            .unwrap_or_default();

        if !author.is_empty() {
            *author_commits.entry(author).or_insert(0) += 1;
        }

        // Get first parent before commit is dropped
        let next_parent = commit.parent_ids().next().map(|pid| pid.detach());
        match next_parent {
            Some(pid) => current_id = pid,
            None => break,
        }
    }

    // Create team entries from top contributors
    if !author_commits.is_empty() {
        let mut sorted: Vec<(String, usize)> = author_commits.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        for (author, _) in sorted.iter().take(5) {
            model.teams.push(Team {
                name: author.clone(),
                owns: Vec::new(),
                contact: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codeowners_match_basic_patterns() {
        assert!(codeowners_match("*", "anything/goes"));
        assert!(codeowners_match("*.rs", "foo/bar/main.rs"));
        assert!(!codeowners_match("*.rs", "foo/bar/main.toml"));

        assert!(codeowners_match("crates/api", "crates/api/Cargo.toml"));
        assert!(codeowners_match("crates/api", "crates/api"));
        assert!(!codeowners_match("crates/api", "crates/worker/Cargo.toml"));

        assert!(codeowners_match("/services/", "services/auth/main.go"));
        assert!(codeowners_match("crates/*", "crates/api"));
        assert!(!codeowners_match("crates/*", "crates/api/src/main.rs"));

        assert!(codeowners_match("src/**", "src/lib/x/y.rs"));
        assert!(codeowners_match("**/test", "foo/bar/test"));
        assert!(codeowners_match("**/test", "test"));
    }

    #[test]
    fn codeowners_last_rule_wins() {
        let text = r#"
# Default
*                 @org/platform

# Overrides
/frontend/        @org/web
/frontend/api/    @org/api-team
"#;
        let rules = parse_codeowners(text);
        assert_eq!(rules.len(), 3);

        let r = last_matching_rule(&rules, "frontend/api/routes.ts").unwrap();
        assert_eq!(r.owners, vec!["@org/api-team".to_string()]);

        let r = last_matching_rule(&rules, "frontend/home.tsx").unwrap();
        assert_eq!(r.owners, vec!["@org/web".to_string()]);

        let r = last_matching_rule(&rules, "backend/main.rs").unwrap();
        assert_eq!(r.owners, vec!["@org/platform".to_string()]);
    }

    #[test]
    fn normalise_team_owner() {
        assert_eq!(normalise_owner("@org/platform"), "platform");
        assert_eq!(normalise_owner("@alice"), "alice");
        assert_eq!(normalise_owner("alice@example.com"), "alice@example.com");
    }

    #[test]
    fn codeowners_populates_teams_from_container_sources() {
        use crate::model::{Element, ElementKind, Model};

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".github")).unwrap();
        std::fs::write(
            root.join(".github/CODEOWNERS"),
            r#"
*                 @org/platform
/crates/api/      @org/api-team
/crates/worker/   @org/workers
"#,
        )
        .unwrap();

        let mut model = Model::default();
        for (id, path) in [
            ("api", "crates/api/Cargo.toml"),
            ("worker", "crates/worker/Cargo.toml"),
            ("misc", "tools/build.rs"),
        ] {
            let mut el = Element::new(id, ElementKind::Container, id);
            el.properties.insert("forge:source".into(), path.into());
            model.add_element(el);
        }

        let matched = scan_codeowners(&mut model, root);
        assert!(matched);

        let api_team = model
            .teams
            .iter()
            .find(|t| t.name == "api-team")
            .expect("api-team created");
        assert_eq!(api_team.owns, vec!["api".to_string()]);

        let workers = model
            .teams
            .iter()
            .find(|t| t.name == "workers")
            .expect("workers created");
        assert_eq!(workers.owns, vec!["worker".to_string()]);

        let platform = model
            .teams
            .iter()
            .find(|t| t.name == "platform")
            .expect("platform created");
        assert_eq!(platform.owns, vec!["misc".to_string()]);
    }

    #[test]
    fn scan_current_repo() {
        // Test against the forge repo itself
        let mut model = Model::default();
        let config = AnalyzeConfig::default();
        // Git root is one level up from the forge crate directory
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        scan(&mut model, &root, &config);

        // Should find at least the main branch
        let branches: Vec<_> = model
            .elements
            .values()
            .filter(|e| e.kind == ElementKind::Branch)
            .collect();
        assert!(!branches.is_empty(), "should find at least one branch");

        // Should detect a branching strategy
        let has_strategy = branches
            .iter()
            .any(|b| b.properties.contains_key("strategy"));
        assert!(has_strategy, "should infer a branching strategy");
    }
}

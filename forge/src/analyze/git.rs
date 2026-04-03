//! Git scanner — infers branching strategy, active branches, and contributor
//! ownership from git history using gix.

use std::collections::HashMap;
use std::path::Path;

use crate::model::*;

use super::AnalyzeConfig;

pub fn scan(model: &mut Model, root: &Path, _config: &AnalyzeConfig) {
    let repo = match gix::open(root) {
        Ok(r) => r,
        Err(_) => return, // not a git repo, skip silently
    };

    scan_branches(model, &repo);
    scan_contributors(model, &repo, root);
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
    trunk.tags.push("inferred".into());
    trunk.tags.push("trunk".into());
    model.add_element(trunk);

    // Create feature branch pattern if detected
    if has_feature {
        let feat_id = format!("{}.feature", strategy_id);
        let mut feat = Element::new(&feat_id, ElementKind::Branch, "feature/*");
        feat.properties
            .insert("strategy".into(), strategy_id.clone());
        feat.properties
            .insert("branchesFrom".into(), trunk_id.clone());
        feat.properties
            .insert("mergesInto".into(), trunk_id.clone());
        feat.tags.push("inferred".into());
        model.add_element(feat);
        model.add_relationship(Relationship {
            frm: trunk_id.clone(),
            to: feat_id.clone(),
            label: "branches from".into(),
            technology: None,
        });
        model.add_relationship(Relationship {
            frm: feat_id,
            to: trunk_id.clone(),
            label: "merges into".into(),
            technology: None,
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
            .insert("branchesFrom".into(), trunk_id.clone());
        dev.tags.push("inferred".into());
        model.add_element(dev);
    }

    // Create release branch pattern for git-flow
    if has_release {
        let rel_id = format!("{}.release", strategy_id);
        let mut rel = Element::new(&rel_id, ElementKind::Branch, "release/*");
        rel.properties
            .insert("strategy".into(), strategy_id.clone());
        rel.tags.push("inferred".into());
        model.add_element(rel);
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

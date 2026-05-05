# Custom Branching Patterns

This document demonstrates how to configure custom branching patterns for your organization's git workflows.

## Overview

Forge's git analyzer can detect common branching strategies (trunk-based, git-flow, github-flow) automatically. However, many organizations use their own branch naming conventions. The `BranchPattern` configuration allows you to define custom patterns that match your team's workflow.

## Configuration

Custom branch patterns can be configured programmatically via the `AnalyzeConfig` struct:

```rust
use forge::analyze::{AnalyzeConfig, BranchPattern};

let mut config = AnalyzeConfig::default();
config.branch_patterns = vec![
    BranchPattern {
        role: "trunk".to_string(),
        pattern: "main".to_string(),
        strategy: "custom-workflow".to_string(),
    },
    BranchPattern {
        role: "task".to_string(),
        pattern: "task/*".to_string(),
        strategy: "custom-workflow".to_string(),
    },
    BranchPattern {
        role: "bugfix".to_string(),
        pattern: "bugfix/*".to_string(),
        strategy: "custom-workflow".to_string(),
    },
];
```

## Pattern Format

### Exact Match
Use an exact branch name to match a specific branch:
```
pattern: "main"
pattern: "develop"
pattern: "staging"
```

### Glob Patterns
Use glob patterns with `*` to match multiple branches:

- **Prefix matching**: `feature/*` matches `feature/login`, `feature/signup`, etc.
- **Suffix matching**: `*/hotfix` matches `team-a/hotfix`, `team-b/hotfix`, etc.

## Role Types

The `role` field categorizes the branch type:

- **trunk/main**: The primary integration branch
- **feature**: Feature development branches
- **bugfix**: Bug fix branches
- **hotfix**: Emergency fix branches
- **release**: Release preparation branches
- **develop**: Development integration branch
- **task**: Task-specific branches (e.g., JIRA ticket branches)

Branches with roles `feature`, `develop`, `bugfix`, `hotfix`, or `task` automatically create relationships with the trunk branch (branches from trunk, merges into trunk).

## Example: JIRA-based Workflow

For teams using JIRA ticket numbers in branch names:

```rust
config.branch_patterns = vec![
    BranchPattern {
        role: "trunk".to_string(),
        pattern: "main".to_string(),
        strategy: "jira-workflow".to_string(),
    },
    BranchPattern {
        role: "task".to_string(),
        pattern: "task/*".to_string(),  // task/PROJ-123
        strategy: "jira-workflow".to_string(),
    },
    BranchPattern {
        role: "bugfix".to_string(),
        pattern: "bugfix/*".to_string(),  // bugfix/PROJ-456
        strategy: "jira-workflow".to_string(),
    },
];
```

## Example: Team-based Workflow

For teams that use team prefixes:

```rust
config.branch_patterns = vec![
    BranchPattern {
        role: "trunk".to_string(),
        pattern: "production".to_string(),
        strategy: "team-workflow".to_string(),
    },
    BranchPattern {
        role: "feature".to_string(),
        pattern: "*/feature".to_string(),  // team-a/feature, team-b/feature
        strategy: "team-workflow".to_string(),
    },
];
```

## Behavior

When custom branch patterns are configured:

1. **Pattern Detection**: The git scanner checks all local branches against your patterns
2. **Strategy Detection**: Only strategies with at least one matching branch are created
3. **Branch Creation**: For each matching pattern, a branch element is created in the model
4. **Relationship Creation**: Non-trunk branches automatically get "branches from" and "merges into" relationships with the trunk
5. **Fallback**: If no custom patterns are configured, Forge uses its built-in pattern detection

## Benefits

- **Match Your Workflow**: Model your actual branching strategy, not a standardized one
- **Accurate Documentation**: Generated diagrams reflect your team's conventions
- **Flexibility**: Support for multiple strategies in the same repository
- **Pattern Reuse**: Define patterns once, apply across multiple repositories

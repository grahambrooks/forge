# Linting

`forge check` validates a model against a set of architectural rules
and reports violations. It's designed to run in CI and fail builds on
anything graver than a configurable severity threshold.

## The minimum viable run

```bash
forge check --source architecture.forge
```

Every rule runs, `warning` and `error` severities are reported by
default, and the exit code is non-zero if anything at or above
`warning` fires.

Output looks like this:

```
[warning] missing-descriptions (payments.cache): Container 'Session Cache' has no description
[warning] missing-technology (billing): Container 'billing' has no technology
[error]   database-direct-access (customer): Person 'Customer' directly accesses database 'db'
```

## Built-in rules

Eight rules ship with forge. See
[Linter rules reference](../reference/linter-rules.md) for the
detailed semantics of each.

| Rule | Severity | What it catches |
| --- | --- | --- |
| `missing-descriptions` | warning | Containers, components, and systems without a `description` |
| `missing-technology` | warning | Containers and components without a `technology` |
| `orphaned-elements` | warning | Elements with no relationships in or out |
| `dependency-cycles` | error | Cycles in the container/component dependency graph |
| `database-direct-access` | error | Persons or external systems directly reaching a `database`-tagged element |
| `chatty-coupling` | info | Pairs with more than N relationships between them |
| `gate-coverage` | warning | Pipeline stages deploying to prod without an approval gate |
| `empty-views` | info | Views that resolve to zero elements |

## Output formats

Three formats, chosen with `--format`:

```bash
forge check --format text      # default, human-readable
forge check --format json      # structured, for CI post-processing
forge check --format sarif     # SARIF 2.1.0 for GitHub Code Scanning
```

SARIF is the interesting one: drop the output into a GitHub Code
Scanning upload step and violations show up on PRs as annotated
comments, exactly like security scanner findings.

```yaml
- name: Run forge check
  run: forge check --format sarif > forge.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: forge.sarif
```

## Severity threshold

`--severity` sets the minimum level reported. Useful when you want to
track `info`-level findings but not fail the build on them:

```bash
# Fail only on errors
forge check --severity error

# Report everything including info
forge check --severity info
```

## Custom rules

Teams often want to express constraints that don't fit any built-in
rule: "no services outside the `platform` team may call the ledger
database", "every container in the `pci` trust boundary must have
`encryption` in its tags", and so on.

Write a `.forge-rules` file next to your model:

```forge-rules
rule "pci-requires-encryption" {
  severity error
  message "Containers in pci boundary must have 'encryption' tag"
  forall container where in_boundary(pci) {
    require has_tag("encryption")
  }
}

rule "ledger-is-platform-only" {
  severity error
  message "Only platform team services may talk to ledger-db"
  forall relationship where target(ledger-db) {
    require source_owned_by("platform")
  }
}
```

Point `forge check` at the rules file:

```bash
forge check --rules team-rules.forge-rules
```

The full custom-rule DSL is documented alongside the linter reference.
A working example lives at
[`forge/examples/team-rules.forge-rules`](../../forge/examples/team-rules.forge-rules).

## In CI

A typical GitHub Actions step:

```yaml
- name: Lint architecture
  run: |
    forge check \
      --source architecture.forge \
      --format sarif \
      --rules .forge-rules \
      > forge.sarif
```

And a stricter variant that fails on any finding:

```yaml
- name: Lint architecture (strict)
  run: forge check --severity info --source architecture.forge
```

## See also

- [Linter rules reference](../reference/linter-rules.md) — the full
  semantics of each built-in rule
- [Your first model](first-model.md) — write a tiny model and run
  `forge check` against it
- [CI integration](../workflows/ci-integration.md) — a complete
  workflow combining `analyze --merge` and `check`

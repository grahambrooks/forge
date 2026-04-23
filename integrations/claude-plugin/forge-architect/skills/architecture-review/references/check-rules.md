# Forge check rules

`forge_check` runs the built-in rules below. Each returns zero or more `Violation`s with severity, rule id, offending element id, and message.

## `missing-description` — info

An element kind that ought to explain itself (`Person`, `System`, `Container`, `Component`) has no `description`. Non-fatal but harmful for docs. Often trivially fixable by reading the source and adding a sentence.

## `missing-technology` — info

A `Container` or `Component` with no `technology`. Downstream tools (tech-stack roll-up, deployment rendering) degrade silently. Inferable from Dockerfile `FROM`, `package.json` "dependencies", `Cargo.toml` edition, etc.

## `orphan-container` — warning

A `Container` with no inbound **and** no outbound relationships. Two common causes:

1. The scanner found a build manifest but the `semantic` scanner didn't connect anything to it (e.g. a CLI tool embedded in a monorepo).
2. It's genuinely unreachable — worth asking whether it should still exist.

## `dependency-cycle` — error

A directed cycle of relationships between structural elements. Architectural debt that usually requires a design decision, not a quick fix. Reported with the full cycle path.

## `direct-database-access` — warning

A `Container` that isn't the database's nominal owner has an outgoing edge to an element tagged `database`. Often the right pattern is an intermediary service — but legacy systems legitimately share access. Review case-by-case.

## `data-class-boundary` — error

A container with `data-class "pii"` (or `financial`, `secret`) is not a member of a `trust-boundary` at or above `pci` level. This is the rule that most often surfaces actual compliance issues.

## `gate-coverage` — warning

A `Pipeline` that contains a deploy-to-production `Stage` but no `Gate`. The typical fix is to add `gate "manual-approval"` in the DSL.

## `chatty-coupling` — info

The same ordered pair of elements has >3 relationships between them. Signals that the interaction is poorly abstracted; usually a hint that an interface needs to be introduced. Can be noisy on data-intensive services — ignore when deliberate.

## `empty-view` — info

A declared view would render nothing given the current model. Usually a stale view declaration; prune from the `.forge`.

## Custom rules

Users can supplement with a `.forge-rules` file:

```
rule no-shared-db {
  severity warning
  for container tag="database"
  expect-max-incoming 1
  message "Databases should have a single owning service"
}
```

Pass via `forge check --rules team-rules.forge-rules`. The MCP server does not currently load custom rules; use the CLI when this matters.

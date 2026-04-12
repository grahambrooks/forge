# Linter rules

`forge check` runs eight built-in rules on every model. This page is
the authoritative spec for each rule — what it fires on, what
severity it uses, and how to suppress or fix violations.

Custom rules in `.forge-rules` can complement these built-ins for
team-specific constraints; see [Linting](../user-guide/linting.md)
for the tutorial on custom rules.

## Severity levels

| Level | Meaning |
| --- | --- |
| `info` | Observation — useful to know but not worth failing a build over |
| `warning` | Something the team should fix, but not blocking |
| `error` | Architectural rule violation that fails CI |

`forge check --severity <level>` sets the minimum level reported.
Default is `warning`. Exit code is non-zero if any violation at or
above the threshold fires.

---

## `missing-descriptions`

**Severity:** warning

**Fires when:** a `container`, `component`, or `system` element has
no `description` field.

**Rationale:** architecture diagrams are a communication tool. An
element with no description is a box with a label — useful to its
author, opaque to everyone else.

**How to fix:** add a one-line description to every non-trivial
element. Good descriptions answer "why does this exist?"; bad ones
repeat the name:

```forge
// Bad — description adds nothing
api = container "API" {
  description "The API"
}

// Good — tells the reader what this thing actually does
api = container "API" {
  description "HTTPS + gRPC gateway for the payments platform"
}
```

**Suppressing:** none. If a container genuinely doesn't need a
description, remove the container. If it's a placeholder, leave the
warning until you fill it in.

---

## `missing-technology`

**Severity:** warning

**Fires when:** a `container` or `component` has no `technology` field.

**Rationale:** the technology label tells readers what language,
runtime, or framework runs inside. It's also what the [`code`
scanner](scanners.md#code) tries to infer automatically. If you see
this on an inferred container, the scanner couldn't match any
framework — worth a quick hand-edit.

**How to fix:**

```forge
api = container "API" {
  description "HTTPS + gRPC gateway"
  technology "Rust / Axum"   // add this
}
```

**Suppressing:** none. If the container is not really code (e.g.
an abstract grouping), consider modelling it as a `system` or a
`deploymentNode` instead.

---

## `orphaned-elements`

**Severity:** warning

**Fires when:** a container or component has no relationships in
or out.

**Rationale:** an element with no connections is either dead weight
or missing context. Either delete it or connect it to whatever
actually uses it.

**How to fix:** add at least one relationship, or delete the element.

**Common cause:** sketching out a future component without wiring
it up yet. Fine in a draft; fix before merging the PR.

---

## `dependency-cycles`

**Severity:** error

**Fires when:** the container/component relationship graph has a
cycle.

**Rationale:** cycles in the architecture graph are almost always a
mistake — they mean services can't be deployed independently,
testing in isolation is impossible, and the system can deadlock at
startup. Cycles below module boundaries (component → component)
might be intentional; cycles across container boundaries almost
never are.

**How to fix:** break the cycle. Usually one of these works:

1. Extract a shared library or interface that both sides depend on
   (downward dependency, not a cycle).
2. Introduce an event-driven boundary — producer publishes, consumer
   subscribes; no direct call back.
3. Invert one direction by dependency injection.

**Example violation:**

```forge
a -> b "calls"
b -> c "calls"
c -> a "calls"   // cycle: a → b → c → a
```

**Suppressing:** there's no suppression syntax. If a cycle is
genuinely intentional (rare), file an issue — we should probably
add severity overrides.

---

## `database-direct-access`

**Severity:** error

**Fires when:** a `person` or `system` (that isn't the owning
system) has a direct relationship to an element tagged `database`.

**Rationale:** persons should never touch databases directly — that's
what an API is for. External systems reaching into your database is
a coupling disaster. The rule catches both at model-review time.

**How to fix:** route the relationship through an API container.

```forge
// Bad
customer -> db "queries"

// Good
customer -> api "queries"
api      -> db  "reads" "SQL"
```

**Exception:** a container *inside* the same system can of course
read/write the database directly; this rule only fires when the
source is a `person` or an `external` system.

---

## `chatty-coupling`

**Severity:** info

**Fires when:** two elements have more than N relationships between
them (N currently fixed at 3, subject to change).

**Rationale:** a pair with a dozen arrows back and forth is either a
modelling artefact (you've flattened multiple distinct interactions
into micro-relationships) or a design smell (the boundary between
the two is the wrong shape).

**How to fix:**

- **Modelling case:** collapse related relationships into a single
  labelled one. Ten arrows labelled "CreateOrder", "UpdateOrder",
  "DeleteOrder", etc. become one arrow labelled "manages orders".
- **Design case:** reconsider the boundary. If everything in one
  side always changes together with everything on the other side,
  they're probably one module, not two.

**Suppressing:** accepted as info for now — bump `--severity` to
`warning` if you want it surfaced in CI.

---

## `gate-coverage`

**Severity:** warning

**Fires when:** a pipeline stage has `environment: production` (or
equivalent) but no approval `gate`.

**Rationale:** deploys to production should not be one-click. A gate
is where the human approval or automated guardrail lives.

**How to fix:**

```forge
// Bad — no gate
prod = stage "Deploy Production" {
  needs staging
  environment production
}

// Good
prod = stage "Deploy Production" {
  needs staging
  environment production
  gate "manual-approval" {
    approvers "platform-team"
  }
}
```

The gate doesn't have to be manual — an automated "green signal"
gate from a canary check also satisfies the rule. What matters is
that the stage has something called `gate` on it.

---

## `empty-views`

**Severity:** info

**Fires when:** a `views { … }` entry resolves to zero elements.

**Rationale:** an empty view renders to a blank SVG. Usually means
you wrote `include *` at the wrong scope, or referenced an element
id that no longer exists.

**How to fix:**

1. Check the scope. `systemContext payments` includes only things
   in the `payments` system.
2. Check the includes. `include foo bar` silently skips missing
   ids; if both `foo` and `bar` are typos, the view is empty.
3. Check the filter. If your view has a `where` clause that
   matches nothing, relax it.

---

## Rule ordering and output

Violations are sorted by:

1. Severity descending (`error` first, then `warning`, then `info`)
2. Rule name ascending (alphabetical)
3. Element id ascending

This makes violation output deterministic, which is important for
diffing CI output across commits.

## Custom rules

For team-specific constraints (trust-boundary enforcement, naming
conventions, ownership requirements, etc.) write a `.forge-rules`
file. See [`forge/examples/team-rules.forge-rules`](../../forge/examples/team-rules.forge-rules)
for a working example and [Linting](../user-guide/linting.md) for
the DSL.

## Source

Every built-in rule lives in [`forge/src/check.rs`](../../forge/src/check.rs).
Each is a small focused function — the whole file is under 400
lines. Adding a new rule is a matter of writing one function and
calling it from `check()`.

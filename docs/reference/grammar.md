# Grammar reference

This is the formal grammar for the `.forge` DSL, in W3C-style EBNF.
It is the normative spec of what the parser in
[`forge/src/parser.rs`](../../forge/src/parser.rs) accepts.

For a friendlier tour with worked examples, see
[DSL quick reference](dsl-quickref.md). For the CLI that drives the
parser, see [CLI reference](cli.md).

## How to read the grammar

W3C EBNF conventions used throughout:

| Notation | Meaning |
| --- | --- |
| `Rule ::= ...` | Definition of a production |
| `"keyword"` | Literal terminal — matched verbatim in source |
| `foo` | Reference to another production |
| `foo bar` | Juxtaposition means concatenation |
| `foo \| bar` | Alternation — either one matches |
| `foo?` | Zero or one occurrence |
| `foo*` | Zero or more occurrences |
| `foo+` | One or more occurrences |
| `(foo bar)` | Grouping |
| `/* … */` | Comment, not part of the grammar |

Whitespace (spaces, tabs, newlines) and `//`-style line comments are
allowed between any two tokens. They are not represented in the
grammar rules below; assume `Ws` is implicitly permitted between
every terminal and every production reference.

All keywords are ASCII lowercase, **kebab-case** where compound
(e.g. `tech-stack`, `system-context-view`). Identifiers are also
kebab-case by convention but the parser accepts any mix of letters,
digits, underscores, hyphens, and dots.

## Lexical rules

```ebnf
Ws          ::= (" " | "\t" | "\n" | "\r" | Comment)*
Comment     ::= "//" (~"\n")*             /* single-line only */

Ident       ::= IdentStart IdentCont*
IdentStart  ::= [a-zA-Z_]
IdentCont   ::= [a-zA-Z0-9_\-./*]         /* dot allows scoped refs, slash allows globs */

String      ::= '"' StringChar* '"'
StringChar  ::= ~('"' | '\\') | '\\' AnyChar
AnyChar     ::= /* any Unicode scalar */

Integer     ::= [0-9]+
```

Note the identifier rule: because `.`, `/`, and `*` are allowed
inside identifiers, forge can parse scoped references like
`payments.api`, glob patterns like `feature/*`, and
directory-style scoped ids all as a single `Ident` token. The
price is that `1.2` is also an `Ident` (not a float) — the parser
uses `Integer` only in contexts where a bare number is expected
(step ordering, grid dimensions).

## Top-level document

A forge document is a single `forge` block. Exactly one per file.

```ebnf
Document     ::= ForgeBlock
ForgeBlock   ::= "forge" String "{" TopLevelStmt* "}"

TopLevelStmt ::= DescriptionStmt
              | ModelBlock
              | ProcessBlock
              | DeploymentBlock
              | TechStackBlock
              | DataModelBlock
              | TrustBoundariesBlock
              | TeamsBlock
              | ApisBlock
              | EventFlowsBlock
              | EnvConfigBlock
              | SlosBlock
              | DependenciesBlock
              | ViewsBlock
              | DocsBlock

DescriptionStmt ::= "description" String
```

Every top-level block is optional except nothing forces you to
include any of them — an empty `forge "X" {}` is a valid (if
useless) document. Order within `TopLevelStmt*` is irrelevant;
the parser processes them in the order they appear but the
semantic model doesn't care.

## Model block

Architecture: persons, systems, containers, components, and the
relationships between them.

```ebnf
ModelBlock      ::= "model" "{" ModelStmt* "}"
ModelStmt       ::= ElementBinding | Relationship | DescriptionStmt

ElementBinding  ::= Ident "=" ElementKind String ElementBody?
ElementKind     ::= "person" | "system" | "container" | "component"

ElementBody     ::= "{" ElementBodyStmt* "}"
ElementBodyStmt ::= "description" String
                  | "technology" String
                  | "tags" String+
                  | "data-class" String+
                  | ElementBinding          /* nested children */
                  | Relationship

Relationship    ::= Ident "->" Ident String? String?
                    /*                 label   technology */
```

**Scoping.** An `ElementBinding` inside another element's
`ElementBody` becomes a child: its id is prefixed with the parent's
id (e.g. `payments.api`). A top-level `Relationship` in a `model`
block may reference any element by its fully-qualified id or by a
local shorthand that resolves against the surrounding scope.

**Kind legality.** The parser does not enforce C4 level nesting —
technically a `container` may contain another `container` — but
downstream rendering assumes `system > container > component`.
Step outside that ordering at your own risk.

## Process block

Delivery processes: repositories, branching strategies, CI/CD
pipelines.

```ebnf
ProcessBlock       ::= "process" "{" ProcessBinding* "}"
ProcessBinding     ::= Ident "=" ProcessKind String ProcessBody?
ProcessKind        ::= "repository" | "strategy" | "pipeline"

ProcessBody        ::= RepositoryBody | StrategyBody | PipelineBody

RepositoryBody     ::= "{" RepositoryStmt* "}"
RepositoryStmt     ::= "url" String
                     | "system" Ident

StrategyBody       ::= "{" BranchBinding* "}"
BranchBinding      ::= Ident "=" "branch" String BranchBody?
BranchBody         ::= "{" BranchStmt* "}"
BranchStmt         ::= "protection" String+
                     | "branches-from" Ident
                     | "merges-into" Ident

PipelineBody       ::= "{" PipelineStmt* "}"
PipelineStmt       ::= TriggersStmt | StageBinding
TriggersStmt       ::= "triggers" Ident String              /* repo event */
StageBinding       ::= Ident "=" "stage" String StageBody?
StageBody          ::= "{" StageStmt* "}"
StageStmt          ::= "needs" Ident
                     | "step" String
                     | "environment" Ident
                     | "gate" String GateBody?
GateBody           ::= "{" GateProp* "}"
GateProp           ::= Ident String                         /* free-form key/value */
```

**Process binding form.** Every process element — repositories,
strategies, pipelines — uses the same `id = kind "Name"` form as
model elements. This is one of the consistency fixes applied in
the current DSL; see the "Consistency findings" section below.

## Deployment block

Infrastructure topology: nested deployment nodes with container
instance bindings.

```ebnf
DeploymentBlock    ::= "deployment" Ident String "{" DeploymentNode* "}"
                      /*             ^env-id  ^display-name           */

DeploymentNode     ::= "node" Ident String "{" DeploymentNodeStmt* "}"
                      /*     ^node-id ^display-name               */

DeploymentNodeStmt ::= "technology" String
                     | "description" String
                     | "tags" String+
                     | "instances" Ident                  /* count, e.g. "3" or "auto" */
                     | DeploymentNode                     /* nested */
                     | "instance" Ident                   /* references a container id */
```

`deployment production "Production"` binds the environment id
`production` in the id table. Views reference it via
`deployment-view production`.

## Tech stack

A categorised technology inventory.

```ebnf
TechStackBlock   ::= "tech-stack" "{" TechCategory* "}"
TechCategory     ::= "category" String "{" TechEntry* "}"
TechEntry        ::= "tech" String TechEntryBody?
TechEntryBody    ::= "{" TechEntryProp* "}"
TechEntryProp    ::= "version" String
                   | "purpose" String
```

## Data model

Entity-relationship modelling. Entities are bound to containers
that store them via `owner`.

```ebnf
DataModelBlock   ::= "data-model" "{" DataStmt* "}"
DataStmt         ::= Entity | DataRelationship

Entity           ::= "entity" String EntityBody?
EntityBody       ::= "{" EntityStmt* "}"
EntityStmt       ::= Field | "owner" Ident
Field            ::= "field" String String String*
                    /*            ^name  ^type   ^constraints... */

DataRelationship ::= "relationship" String String RelBody?
                    /*              ^from   ^to                  */
RelBody          ::= "{" DataRelProp* "}"
DataRelProp      ::= "label" String
                   | "cardinality" String
```

**Field syntax.** A `Field` production is the only place in the
grammar that takes three positional quoted strings (name, type,
and an optional list of constraint strings). It's kept that way
because it matches the way schema definitions are usually written.

## Trust boundaries

Security zones that group containers.

```ebnf
TrustBoundariesBlock ::= "trust-boundaries" "{" Boundary* "}"
Boundary             ::= "boundary" String "{" BoundaryStmt* "}"
BoundaryStmt         ::= "level" String                 /* public / dmz / internal / pci */
                       | "includes" Ident               /* one per line, repeatable */
```

## Teams

Team definitions and ownership attributions.

```ebnf
TeamsBlock ::= "teams" "{" Team* "}"
Team       ::= "team" String "{" TeamStmt* "}"
TeamStmt   ::= "owns" Ident                      /* one per line, repeatable */
             | "contact" String
```

## APIs

Endpoint catalogs attached to containers.

```ebnf
ApisBlock    ::= "apis" "{" Api* "}"
Api          ::= "api" Ident "{" Endpoint* "}"
                /*    ^container-id          */
Endpoint     ::= "endpoint" String String EndpointBody?
                /*          ^method ^path               */
EndpointBody ::= "{" EndpointProp* "}"
EndpointProp ::= "description" String
               | "request" String
               | "response" String
```

**Method + path.** Endpoints carry both a method and a path as
separate quoted strings — `endpoint "POST" "/payments"` — rather
than the v1 single-string form. For RPC-style endpoints that don't
have a path, use the method name as the second argument:
`endpoint "RPC" "ProcessPayment"`.

## Event flows

Asynchronous message flows between publishers and subscribers.

```ebnf
EventFlowsBlock ::= "event-flows" "{" Flow* "}"
Flow            ::= "flow" String "{" FlowStmt* "}"
FlowStmt        ::= "topic" String
                  | "description" String
                  | "publisher" Ident                /* repeatable */
                  | "subscriber" Ident               /* repeatable */
```

## Environment config

Per-environment configuration values. The only block where keys
are user-defined identifiers rather than DSL vocabulary.

```ebnf
EnvConfigBlock ::= "env-config" "{" Env* "}"
Env            ::= "env" String "{" ConfigEntry* "}"
ConfigEntry    ::= Ident String                     /* KEY "value" */
```

## SLOs

Per-container service-level objectives.

```ebnf
SlosBlock   ::= "slos" "{" Slo* "}"
Slo         ::= "slo" Ident "{" SloStmt* "}"
              /*     ^container-id              */
SloStmt     ::= "latency" String
              | "availability" String
              | "error-budget" String
```

## Dependencies

External systems and SaaS the model depends on.

```ebnf
DependenciesBlock ::= "dependencies" "{" Dependency* "}"
Dependency        ::= "dependency" String "{" DependencyStmt* "}"
DependencyStmt    ::= "kind" String
                    | "criticality" String
                    | "url" String
                    | "description" String
```

## Views block

All renderable views. Every view kind ends in `-view`. Scoped
views take a bare id reference to the element or process
collection they visualize; unscoped views take only the output
key.

```ebnf
ViewsBlock         ::= "views" "{" View* "}"
View               ::= ScopedView | UnscopedView

ScopedView         ::= ScopedViewKind Ident String ViewBody?
                      /*              ^scope ^key              */
UnscopedView       ::= UnscopedViewKind String ViewBody?
                      /*                 ^key              */

ScopedViewKind     ::= "system-context-view"
                     | "container-view"
                     | "component-view"
                     | "pipeline-view"
                     | "deployment-view"
                     | "branching-view"
                     | "dynamic-view"

UnscopedViewKind   ::= "tech-stack-view"
                     | "data-model-view"
                     | "trust-boundary-view"
                     | "team-view"
                     | "api-catalog-view"
                     | "event-flow-view"
                     | "composite-view"

ViewBody           ::= "{" ViewStmt* "}"
ViewStmt           ::= IncludeStmt
                     | "auto-layout" ("lr" | "tb")
                     | "title" String
                     | Animation
                     | OrderedRelationship          /* dynamic-view only */
                     | CompositeStmt                /* composite-view only */

IncludeStmt        ::= "include" "*"                /* include everything in scope */
                     | "include" Ident+             /* list of ids */

OrderedRelationship ::= Integer "." Ident "->" Ident String String?
                       /*       ^    ^src        ^dst    ^label ^tech  */

CompositeStmt      ::= "grid" Integer Integer
                     | "cell-size" Integer Integer
                     | "cell" String                /* references another view's key */
```

**View body is optional.** Unlike earlier DSL versions, a view
with nothing to configure beyond the key may omit `{}`:
`tech-stack-view "TechStack"` is a valid single-line view.

**Dynamic views** use a special relationship form inside their
body: `<step>. src -> dst "label" "tech"?`. The leading integer
becomes the relationship's `order` field and drives both the
step-badge rendering and the auto-generated animation frames.

**Composite views** can only be declared with `composite-view`.
The grammar rule above shows the statements that are only legal
inside a composite view body.

## Animation

Frame-based reveal for dynamic-view and any-view walkthroughs.

```ebnf
Animation      ::= "animation" "{" Frame* "}"
Frame          ::= "frame" String "{" FrameStmt* "}"
FrameStmt      ::= "include" "*"
                 | "include" Ident
                 | "include" Ident "->" Ident
                 | Highlight
                 | State
                 | "notes" String

Highlight      ::= "highlight" Ident HighlightTarget? HighlightBody?
HighlightTarget ::= "->" Ident ("->" Ident)*        /* chain of relationship endpoints */
HighlightBody  ::= "{" HighlightProp* "}"
HighlightProp  ::= "color" String
                 | "line-width" Ident                /* numeric ident, e.g. "2.5" */
                 | "label" String

State          ::= "state" Ident String StateBody?
                   /*      ^target ^label           */
StateBody      ::= "{" StateProp* "}"
StateProp      ::= "color" String
                 | "pulse" ("true" | "false")
```

## Docs

Markdown pages to bundle into generated sites.

```ebnf
DocsBlock ::= "docs" "{" DocStmt* "}"
DocStmt   ::= "doc" String String
            /*      ^title  ^path (relative to the .forge file) */
```

## Preprocessor directives

Three directives are resolved by the preprocessor
([`forge/src/preprocess.rs`](../../forge/src/preprocess.rs))
before the parser runs. They can appear anywhere except inside a
string:

```ebnf
Directive     ::= Include | Fragment | Use | If

Include       ::= "!include" String                /* path or glob, relative to the current file */
Fragment      ::= "!fragment" Ident "{" /* content */ "}"
Use           ::= "!use" Ident
If            ::= "!if" Condition "{" /* content */ "}"
Condition     ::= "env" "(" String ")" (("==" | "!=") String)?
```

`!include` accepts both exact paths and glob patterns like
`model/*.forge`. `!fragment`/`!use` define and inline named
snippets; useful for DRYing out repeated fragments across files.
`!if` gates a block on an environment variable test.

## Reserved words

The following strings are keywords and cannot be used as bare
identifiers. Using one as an id (e.g.
`model = container "Model"`) is a parse error.

```
// Top-level blocks
forge description model process deployment tech-stack data-model
trust-boundaries teams apis event-flows env-config slos
dependencies views docs

// Element kinds
person system container component repository branch pipeline
stage gate step node

// Element body
technology tags data-class

// Process
url strategy triggers needs environment protection branches-from
merges-into

// Data model
entity field owner relationship label cardinality

// Trust boundaries
boundary level includes

// Teams
team owns contact

// APIs
api endpoint request response

// Event flows
flow topic publisher subscriber

// Env config
env

// SLOs
slo latency availability error-budget

// Dependencies
dependency kind criticality

// Views
system-context-view container-view component-view pipeline-view
deployment-view branching-view tech-stack-view data-model-view
trust-boundary-view team-view api-catalog-view event-flow-view
dynamic-view composite-view
include auto-layout title animation grid cell cell-size

// Animation
frame highlight state notes color line-width pulse

// Tech stack
tech-stack category tech version purpose

// Docs
doc

// Literals
true false lr tb
```

## Consistency findings

This section documents the design decisions applied during the
consistency review. Every item here is a deliberate choice made
while formalising the grammar; earlier DSL versions had the
inconsistencies described under "Before."

### Keyword casing: all kebab-case

**Before.** Compound block names used camelCase (`techStack`,
`dataModel`, `trustBoundaries`, `eventFlows`, `envConfig`), while
field names were a mix of camelCase (`autoLayout`, `cellSize`,
`branchesFrom`, `mergesInto`, `lineWidth`, `errorBudget`,
`dataClass`) and snake_case (`latency_p99`, `error_budget`).

**After.** Every compound keyword is kebab-case:
`tech-stack`, `data-model`, `trust-boundaries`, `event-flows`,
`env-config`, `auto-layout`, `cell-size`, `branches-from`,
`merges-into`, `line-width`, `error-budget`, `data-class`.
Rationale: kebab-case reads clearly at a glance and visually
distinguishes keywords from Rust-style identifiers readers might
confuse them with.

### View kind suffix: every kind ends in `-view`

**Before.** Five view kinds had no suffix (`systemContext`,
`container`, `component`, `dynamic`, `composite`); nine had the
`View` suffix. The naming difference didn't reflect a semantic
distinction — it was historical drift.

**After.** Every view kind ends in `-view`:
`system-context-view`, `container-view`, `component-view`,
`pipeline-view`, `deployment-view`, `branching-view`,
`dynamic-view`, `composite-view`, `tech-stack-view`,
`data-model-view`, `trust-boundary-view`, `team-view`,
`api-catalog-view`, `event-flow-view`. The suffix resolves the
ambiguity where `container` meant both "the thing" and "a view of
the thing."

### Scope reference: always a bare id

**Before.** Scoped views accepted the scope ref in two
incompatible forms. `systemContext payments` used a bare id;
`pipelineView "payments-ci"` used a quoted string matching a
pipeline's auto-slugged id. The difference came from the fact
that pipelines, strategies, and deployment environments were
declared by string name in the process section.

**After.** Scope references are always bare ids.
`pipeline-view payments-ci`, `deployment-view production`,
`branching-view trunk-based`. To make this work, process-section
elements now use the model-element binding form:
`payments-ci = pipeline "Payments CI"`.

### Process bindings: everything uses `=`

**Before.** Pipelines (`pipeline "name"`), strategies
(`strategy "name"`), and deployment environments
(`deployment "name"`) declared ids implicitly by slugifying the
quoted name. Repositories, in contrast, used the model-style
`repo = repository "name"` form.

**After.** Every process-section element uses
`<id> = <kind> "Display Name"`, matching how model elements
work: `repo = repository "payments-api"`,
`payments-ci = pipeline "Payments CI"`,
`trunk-based = strategy "Trunk-based"`. Deployment is a top-level
block (not inside `process`) but uses the same pattern:
`deployment production "Production" { ... }` — the bare id
follows the block keyword.

### Single-spelling keywords

**Before.** Several keywords accepted two spellings for the same
concept: `instances` / `replicas`, `member` / `includes`,
`latency` / `latency_p99`, `error_budget` / `errorBudget`,
`kind` / `type` (in dependencies).

**After.** One spelling each: `instances`, `includes`, `latency`,
`error-budget`, `kind`. The dropped alternates now produce parse
errors rather than silently being accepted.

### Strict error handling

**Before.** Many parser branches silently skipped unknown keywords
(the `_ =>` arms called `parse_string()` or `skip_block()`). This
let typos like `titel "…"` pass through unnoticed, producing
views without titles.

**After.** Unknown keywords inside a recognised block produce
explicit parse errors: `unknown top-level block '…'`,
`unknown view keyword '…'`, `unknown stage keyword '…'`, and so
on. The silent-skip branches have been removed.

### `include` accepts a list

**Before.** `include a b c` in a view body only captured the
first identifier; the rest were silently eaten by the trailing
lenient branch.

**After.** `include` reads identifiers greedily until it hits a
non-ident character, so `include a b c` collects all three.

### API endpoints: structured method + path

**Before.** `endpoint "GET /payments"` was a single string split
on the first space at parse time. The parser couldn't tell a
missing method from a path that happened to contain a space.

**After.** `endpoint "GET" "/payments"` uses two separate quoted
strings. RPC-style endpoints without a URL path use the procedure
name as the second string: `endpoint "RPC" "ProcessPayment"`.

### `triggers` is tokenised

**Before.** `triggers repo.main on "push"` was parsed by
skip-to-end-of-line — whatever you wrote after `triggers` was
read as prose and discarded.

**After.** `triggers <repo-id> <event-string>` is tokenised:
`triggers repo "push"`. The resolved repo id is stored as a
`triggered_by` property on the pipeline element.

### Removed blocks and keywords

**Before.** `styles { ... }` at the top level was silently
consumed but never parsed or stored. `produces <id> "path"`
inside a stage was parsed and discarded.

**After.** Both are removed. Writing `styles { ... }` or
`produces` inside a stage produces a parse error. Use an external
CSS file for diagram styling (documented in the generate guide)
and annotate pipeline outputs via hand-authored relationships
rather than `produces`.

### View body is optional

**Before.** `parse_view_body` always required `{}`, even for
views like `tech-stack-view "TechStack"` that have no configurable
properties beyond the key.

**After.** The body is optional. A view with nothing to configure
beyond the key can omit the braces entirely.

### Not changed

A handful of softer inconsistencies were deliberately left alone:

- `tags "a" "b"` takes a list on one line but `owns <id>` takes
  one per line. Rationale: `owns` references cross-type elements
  and benefits from line-per-item readability.
- `env "name" { KEY "value" }` uses bare identifiers as keys.
  This is the one block where keys are user-supplied data, not
  DSL vocabulary.
- `pulse true` / `pulse false` uses bare identifiers for
  booleans rather than a proper boolean literal. Introducing a
  boolean type for the two sites that need it would be
  disproportionate.

## Complete worked example

See [`forge/examples/payments.forge`](../../forge/examples/payments.forge)
for a full model that exercises every block in this grammar. For
a minimal walkthrough building the same structure step by step,
see [Your first model](../user-guide/first-model.md).

## See also

- [DSL quick reference](dsl-quickref.md) — friendlier tour with
  inline examples
- [CLI reference](cli.md) — how to invoke the parser
- [Linter rules](linter-rules.md) — the checks applied after
  parsing succeeds
- [`DESIGN.md`](../../DESIGN.md) — the product vision spec
  (richer and more speculative than this grammar)

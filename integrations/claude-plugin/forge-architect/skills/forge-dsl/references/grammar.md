# Forge DSL keyword reference

All keywords are kebab-case. Identifiers are case-sensitive. Strings are double-quoted. Comments use `//`.

## Top level

| Keyword | Purpose |
|---|---|
| `forge "<name>" { ... }` | Workspace root. Exactly one per file tree. |
| `description "..."` | One-line summary. |

## Sections (inside `forge { }`)

| Section | Purpose |
|---|---|
| `model { ... }` | Structural elements and relationships. |
| `process { ... }` | Repositories, strategies, pipelines. |
| `deployment <env> "Label" { ... }` | Topology for one environment. Repeatable. |
| `tech-stack { ... }` | Categorised tech inventory. |
| `data-model { ... }` | Entities, fields, relations. |
| `trust-boundaries { ... }` | Security zones. |
| `teams { ... }` | Team ownership mapping. |
| `views { ... }` | View declarations. |
| `docs { ... }` | Markdown doc references. |

## Structural elements (`model`)

```
<id> = person    "Label"   { ... }
<id> = system    "Label"   { ... }
<id> = container "Label"   { ... }
<id> = component "Label"   { ... }
```

Element body keys:

| Key | Value | Notes |
|---|---|---|
| `description "..."` | one-line | Required by `missing-description` lint. |
| `technology "..."` | string | Required by `missing-technology` lint. |
| `tags "a" "b"` | string list | Free-form. Conventional tags: `database`, `pci`, `public`, `legacy`. |
| `data-class "..." "..."` | string list | Data classification. Triggers `data-class-boundary` rule. |
| `properties { key "value" }` | map | Scanner-set (`dockerfile`, `image`, `ports`, …) but you can add your own. |

## Relationships (`model`)

```
<from> -> <to> "label"                        // label only
<from> -> <to> "label" "technology"           // label + tech
<from> -> <to> "label" { order 2 }            // ordered step (for dynamic views)
```

`from` and `to` are element ids, dotted across scopes (`payments.api`).

## Process (`process`)

```
<id> = repository "<name>" {
  url "https://..."
  system <system-id>
}

<id> = strategy "<name>" {
  <branch-id> = branch "<pattern>" {
    protection "..." "..."
    branches-from <branch-id>
    merges-into   <branch-id>
  }
}

<id> = pipeline "<name>" {
  triggers <repo-id> "push" | "tag" | "schedule:<cron>"
  <stage-id> = stage "<name>" {
    needs <stage-id>                  // DAG edge
    environment <env-id>
    step "<shell command or description>"
    gate "<name>" { approvers "..." }
    artifact "<name>"
  }
}
```

## Deployment (`deployment`)

```
deployment <env-id> "Label" {
  node <id> "<name>" {
    technology "..."
    description "..."
    node <child-id> "..." { ... }     // arbitrary nesting
    instance <container-id>           // binds model ↔ infra
  }
}
```

## Views (`views`)

View kind prefixes all take the form `<kind>-view [scope] "Key" { ... }`:

| Kind | Scope | Purpose |
|---|---|---|
| `system-context-view` | system id | Actors + systems. |
| `container-view` | system id | Containers inside one system. |
| `component-view` | container id | Components inside one container. |
| `pipeline-view` | pipeline id | CI/CD stages/gates. |
| `deployment-view` | environment id | Infra topology. |
| `tech-stack-view` | — | Categorised tech. |
| `branching-view` | strategy id | Git branches. |
| `data-model-view` | — | ERD. |
| `trust-boundary-view` | — | Zones. |
| `team-view` | — | Ownership map. |
| `api-catalog-view` | — | Endpoint inventory. |
| `event-flow-view` | — | Event/message flows. |
| `composite-view` | — | Grid of other views. |

View body keys:

| Key | Value |
|---|---|
| `include *` / `include <id>` / `include <from> -> <to>` | Element/edge filter. |
| `auto-layout tb` / `lr` | Direction. |
| `title "..."` | Rendered title. |
| `animation { frame "..." { ... } ... }` | Ordered walkthrough (dynamic view). |

Frame body:

```
frame "Label" {
  include <id> | *
  include <from> -> <to>
  highlight <target> { color "#..."; line-width 3; label "..." }
  state     <target> { color "#..."; pulse; label "..." }
  notes "..."
}
```

## Data model (`data-model`)

```
entity "<Name>" {
  field "<name>" "<type>" "<constraint>" "<constraint>"    // constraints: PK, FK, NOT NULL, UNIQUE, ...
  owner <container-id>
}

relation "<From>" -> "<To>" "label" "1:N"                  // cardinality: 1:1, 1:N, N:M
```

## Trust boundaries (`trust-boundaries`)

```
boundary "<Name>" <level> {
  member <element-id>
  member <element-id>
}
```

`<level>` is one of `public`, `dmz`, `internal`, `pci`, `restricted`.

## Teams (`teams`)

```
team "<Name>" {
  owns <element-id>
  owns <element-id>
  contact "team@example.com"
}
```

## Tech stack (`tech-stack`)

```
category "<Heading>" {
  tech "<Name>" {
    version "..."
    purpose "..."
  }
}
```

## Docs (`docs`)

```
doc "Title" "path/to/file.md"
```

Paths are resolved relative to the `.forge` file.

## Preprocessor

```
!include "path/to/fragment.forge"        // literal path
!include "fragments/*.forge"             // glob

!fragment name { ... }
!use name

!if env(FLAG) { ... }
```

## Notes

- The parser is strict: missing braces, trailing commas, and unknown keywords are errors with line/column diagnostics.
- Prefer `!include` to one-file monoliths once a model exceeds ~500 lines.
- The LSP provides completion, diagnostics, hover, and go-to-definition — recommend `forge lsp` to the user if they're editing a lot.

---
name: architecture-review
description: Review an existing Forge model — answer architecture questions, check for smells, and summarise a system's structure. Use this when the user asks to "review the architecture", "audit the model", "explain how service X fits in", "find coupling issues", "check for data-classification violations", or drills into a specific container/component in a `.forge` file. Requires a model already loaded (see model-repository to create one).
---

# Architecture Review with Forge

Use this skill to interrogate an already-loaded Forge model. The Forge MCP server exposes the whole graph: containers, components, relationships, pipelines, deployments, trust boundaries, teams, data model, tech stack.

**Don't re-analyze unless asked.** If a `.forge` file exists, trust it — it may contain deliberate hand edits that analyze would miss.

## Before you start

1. Call `forge_overview`. If it returns `empty: true`, the server has no model — hand off to `model-repository` to produce one.
2. Note the counts by kind and the view keys. These shape what questions make sense.
3. Call `forge_check { severity: "info" }` so the standing violations are in your working context. Don't flood the user with them yet — just know what's there.

## Question patterns

### "Explain service X" / "What does X do?"

1. `forge_search { query: "X" }` → get candidate ids.
2. `forge_element_detail { id: "<best match>" }` for the canonical view: children, tech, incoming/outgoing edges.
3. Answer with the relationships in plain English: "X receives HTTPS traffic from the API gateway, writes to Postgres, and publishes payment.settled events to Kafka."

Only render a diagram if the user asked for one — words travel better through chat than SVGs.

### "How does data flow from A to B?"

Walk the relationship graph yourself. Start at A, follow outgoing edges toward B, branch on all plausible paths, stop at depth 4. If there are multiple routes, name them. If there isn't one, say so — it often means the model is incomplete.

### "Find coupling / hotspots"

- `forge_query { kind: "Container" }`, then for each, count incoming+outgoing edges via `forge_element_detail`. Containers with >6 neighbours usually warrant discussion.
- The built-in `chatty-coupling` check catches pairs with many parallel edges — call `forge_check`.

### "Any violations?"

`forge_check { severity: "warning" }` first, then group by rule and summarise. Built-in rules include:

- `missing-description` / `missing-technology` — often model-quality, not architecture-quality.
- `orphan-container` — a service nobody talks to; genuine smell.
- `dependency-cycle` — hard architectural problem.
- `direct-database-access` — containers hitting a `database`-tagged element without going through a service. Common in legacy code.
- `data-class-boundary` — a container touching PII/financial data that isn't inside a PCI/restricted trust boundary. Loud when present, silent when not.
- `gate-coverage` — a pipeline that deploys without a gate.
- `empty-view` — a view that renders nothing.
- `chatty-coupling` — too many parallel edges between the same pair.

Present these in the order **error > warning > info**. For each, give the *count* before examples — "7 containers with no technology set, including api, worker, and queue-processor" — so the user can decide whether to care.

### "What tech are we on?"

Tech stack is first-class: read `forge_overview`'s `counts.tech_categories`. For details, drill by reading the source `.forge` (the MCP doesn't currently expose the tech stack as a tool — use Read on the source path from `forge_overview`).

### "Where does PII live?"

Query containers with the `pii` tag or `data-class`: `forge_query { tag: "pii" }`. Cross-reference with the `data-class-boundary` check output. This is the one time to be loud about violations — data classification is typically what the user cares about when they ask.

## Rendering a view

Only when asked:

```
forge_list_views         // see what exists
forge_render { view_key: "Containers" }
```

The result is SVG as a single string. Save it to a file with Write, then tell the user where it lives — don't dump SVG into the chat. If they want PNG or PDF, explain that current output is SVG-only.

## Changing the model

If the review surfaces fixes the user wants applied:

1. Edit the `.forge` source file directly (the path is in `forge_overview.source`). The DSL is in the `forge-dsl` skill's reference.
2. Call `forge_reload` to pick up the edit.
3. Re-run the relevant check / query so the user sees the new state.

Never edit generated SVGs or the docs site — those are outputs, not inputs.

## Tone

Architecture review earns trust by being specific. Cite element ids. Quote relationship labels. Name the file path when you read the `.forge`. Avoid phrases like "generally speaking" and "it might be worth considering" — the model is right there; either it says something or it doesn't.

See `references/check-rules.md` for each built-in lint rule in detail.

# Merge mode

`forge analyze --merge architecture.forge` is the variant you run once
you've started adding hand-written content to your model. Instead of
overwriting the whole file, it preserves everything tagged as
user-authored and refreshes only the inferred parts.

Without this flag, re-running `analyze` deletes any descriptions or
relationships you added by hand. With it, the loop is safe to wire into
CI — which is the main reason it exists.

## The mental model

Every element in a `.forge` file is either **inferred** (produced by a
scanner) or **authored** (written by a human). Inferred elements carry
the tag `inferred` and a scanner-specific tag like `inferred:code` or
`inferred:semantic`:

```forge
api = container "api" {
  description "Rust crate at services/api"
  technology "Rust / Axum"
  tags "inferred" "inferred:code"
}
```

When you edit this file by hand, you have three options:

1. **Leave the tags and refine the content.** The next `--merge` run
   will still consider this element inferred and replace it with
   whatever the scanner currently produces. Anything you typed into
   `description` gets lost on the next run.
2. **Remove the `inferred` tag.** The element becomes user-owned. Future
   `--merge` runs will skip it entirely. You keep your description, but
   you're also responsible for keeping the technology label up to date.
3. **Add a new element without any `inferred` tag.** Pure hand-authored
   content. Untouched by every future `--merge`.

The rule, in one sentence: **if it's tagged `inferred`, the analyzer
owns it; otherwise it's yours.**

## What --merge does, exactly

Given an existing `architecture.forge` and the fresh result of running
every scanner:

1. Drop every element in the existing model that's tagged `inferred`.
2. Drop every relationship whose endpoints no longer exist (because one
   end was just dropped).
3. Add every element from the fresh scan, skipping any id that a user-
   authored element in the existing model already uses (user ids win
   on collision).
4. Add every fresh relationship whose both endpoints resolve in the
   merged model, skipping exact duplicates.
5. Replace `apis { … }` catalogs for every container the fresh scan
   touched; leave catalogs on untouched containers alone.

Result: user content is untouched; inferred content is refreshed;
stale inferred elements and any edges pointing at them are gone.

## Running it

```bash
forge analyze --merge architecture.forge --out architecture.forge .
```

Yes, `--merge` and `--out` can point at the same file — forge loads
the existing model first, runs the merge in memory, then writes the
result back. You'll see before/after counts in the output:

```
Scanning...
  Elements: 17
  Relationships: 9
Merging into architecture.forge...
  Merged: 23 elements (18 before), 14 relationships (11 before)
  Wrote: architecture.forge
Done.
```

## Id collisions

If you write a container with id `api` and the code scanner also
discovers a container with id `api`, which wins?

- If your element is **not** tagged `inferred`, your element wins.
  The scanner's fresh entry is dropped.
- If your element **is** tagged `inferred` (e.g. you edited the
  generated file without removing the tag), the scanner's fresh entry
  replaces yours.

The consequence: **if you want to override analyzer output for a
specific element, remove its `inferred` tag.** This is the escape
hatch.

Example: the scanner keeps detecting your service as `Rust / Axum`
but you've migrated to `Rust / Actix`. Edit the element, update the
technology, and remove the `inferred` tag. Now your override sticks.

## What happens to stale inferred elements

A common scenario: you once had a service called `payments-v1`, the
scanner created a container for it, and you committed `architecture.forge`.
Later you delete the service from the codebase. What happens on the
next `--merge` run?

The container was tagged `inferred:code`, so step 1 drops it. Step 2
drops every relationship that was pointing at it. Nothing hand-authored
was lost because anything hand-authored didn't have the `inferred` tag.

This is the intended behaviour: the analyzer's output is a reflection
of the code *as it is now*, not an archival record of services that
used to exist.

## api_catalog refresh

API endpoint catalogs are handled specially. If the fresh scan found
any endpoints for a container, its entire catalog is replaced with the
fresh one. If the fresh scan found nothing for a container, its
existing catalog is preserved untouched. This matches the typical
workflow: the semantic scanner is authoritative for routes in source
files, and anything you typed into `apis { … }` by hand on a container
the semantic scanner doesn't see survives.

## When NOT to use --merge

- **The first run on a brand-new repo.** Run `analyze` without
  `--merge` to create the initial file. `--merge` fails if the target
  file doesn't exist.
- **When you want to start over.** Delete `architecture.forge`, run
  `analyze` without `--merge`, commit.

## See also

- [CI integration](../workflows/ci-integration.md) — the end-to-end
  story of wiring `--merge` into GitHub Actions with an auto-PR bot.
- [Provenance and merge semantics](../reference/correlations.md#provenance)
  — the exact rules in reference form.

# Brownfield: analyzing an existing codebase

Use this workflow when a codebase already exists and you want a
forge model that reflects it. You'll bootstrap with `forge analyze`,
refine by hand, then lock it in with merge mode so future re-runs
preserve your edits.

This is the most common way teams adopt forge.

## 1. Bootstrap

From the root of your repo:

```bash
forge analyze --out architecture.forge
```

You'll see something like:

```
Scanning...
  Elements: 23
  Relationships: 11
  Wrote: architecture.forge
Done.
```

What forge found depends on what's in your repo. In a typical
polyglot monorepo with docker-compose and k8s manifests you'll see:

- one `container` per service manifest (`Cargo.toml`, `package.json`,
  `go.mod`, `pyproject.toml`, `pom.xml`, `build.gradle`)
- `technology` labels inferred from the dependency sets
- inter-service `imports` / `calls` relationships
- `container` entries for databases and queues (postgres, redis,
  kafka, …) from docker-compose / k8s
- `team` entries from `.github/CODEOWNERS`
- `pipeline` and `stage` entries from `.github/workflows/*.yml`
- `DeploymentNode`s from k8s manifests
- correlated `uses (DATABASE_URL)` edges between services and their
  database containers

See [Analyzing a codebase](../user-guide/analyzing-a-codebase.md) for
the full per-scanner breakdown.

## 2. Preview what you got

```bash
forge serve --source architecture.forge --port 4000
```

Open the preview and click through the generated views. Expect some
noise — scanners are heuristics, not mind-readers.

## 3. Refine

The starter model is usable but not polished. Over a few sessions
you'll want to:

**Add a top-level `system` wrapper.** Forge analyze produces flat
containers. Wrap related containers in a named system so they render
together:

```forge
payments = system "Payments Platform" {
  description "Handles all card and bank payments"

  // move the inferred containers inside
}
```

Move the existing `api = container "api" { … }` blocks inside the
system braces. They keep their `inferred` tags.

**Refine descriptions.** Analyzer descriptions are generic
("Rust crate at services/api"). Replace them with what the container
actually does, and remove the `inferred` tag on the description only
if you want your prose to survive future merges. Or leave it — the
merge will preserve the description field if you've removed the
`inferred` tags at the element level.

**Add views.** Analyze doesn't emit any views by default. Add at
least a `systemContext` and a `container` view so there's something
to show when you run `forge generate`.

**Cull noise.** Some inferred containers are probably wrong — an
`_inferred_postgresql` that correlation couldn't tie back to a real
service, a manifest in a `tests/fixtures/` directory that isn't
actually a real container. Delete the element from the file; the
next merge won't resurrect it unless the underlying file changes.

## 4. Lock in your edits

Once you start caring about surviving re-runs, switch to merge mode:

```bash
forge analyze \
  --merge architecture.forge \
  --out architecture.forge \
  .
```

This is safe to run over and over. User-authored content survives;
inferred content gets refreshed; stale inferred elements disappear.
See [Merge mode](../user-guide/merge-mode.md) for the exact rules.

## 5. Commit and lint

```bash
git add architecture.forge
forge check --source architecture.forge
git commit -m "Add architecture model"
```

Address the linter warnings now or in a follow-up PR. A fresh
brownfield analyze usually produces a handful of
`missing-descriptions` and `missing-technology` warnings on
containers the scanners weren't confident about.

## 6. Wire up CI

Once the model is committed, add a CI step that re-runs analyze on
every push and opens a PR if anything has drifted:

- Copy [`forge/examples/ci/analyze.yml`](../../forge/examples/ci/analyze.yml)
  to `.github/workflows/analyze.yml` in your repo.
- Make sure "Allow GitHub Actions to create and approve pull requests"
  is enabled (Settings → Actions → General → Workflow permissions).

Full details in [CI integration](ci-integration.md).

## Handling the "analyze keeps undoing my edits" problem

If you find the analyzer rewriting something you clearly hand-authored,
one of three things is happening:

1. **You forgot to remove the `inferred` tag.** The element is still
   considered analyzer-owned.
2. **You used an id the analyzer also generates.** User ids win on
   collision, but only if the user element isn't itself tagged
   `inferred`. Check both conditions.
3. **The scanner is emitting something genuinely wrong.** File a
   forge issue with the scanner name and the minimal fixture that
   reproduces it.

The rule: **if it's tagged `inferred`, the analyzer owns it; if it's
not, it's yours.**

## See also

- [Analyzing a codebase](../user-guide/analyzing-a-codebase.md)
- [Merge mode](../user-guide/merge-mode.md)
- [CI integration](ci-integration.md)
- [Greenfield workflow](greenfield.md) if you're also designing
  something new alongside the existing codebase

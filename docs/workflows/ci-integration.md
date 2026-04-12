# CI integration

This workflow keeps a committed `.forge` model in sync with a living
codebase automatically. On every push to `main`, CI re-runs
`forge analyze --merge`, and if anything has drifted, it opens a pull
request with the refresh.

The net effect: the model never gets stale, and humans only look at
it when something they care about has actually changed.

## Prerequisites

- The repo already has a committed `architecture.forge`. If it doesn't,
  bootstrap it first — see [Brownfield workflow](brownfield.md) steps 1
  through 5.
- "Allow GitHub Actions to create and approve pull requests" is enabled
  in the repo settings (Settings → Actions → General → Workflow
  permissions).
- The default branch is `main`. Adjust the `on.push.branches` field in
  the workflow below if you use something else.

## Drop-in workflow

The forge repo ships a reference workflow at
[`forge/examples/ci/analyze.yml`](../../forge/examples/ci/analyze.yml).
Copy it to `.github/workflows/analyze.yml` in your own repo. The
version below is the same file with inline commentary.

```yaml
name: Analyze architecture

on:
  push:
    branches: [main]
  workflow_dispatch: {}

permissions:
  contents: write        # needed to push the refresh branch
  pull-requests: write   # needed to open the refresh PR

jobs:
  analyze:
    name: Refresh architecture.forge
    runs-on: ubuntu-latest
    steps:
      - name: Check out repository
        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          # Enough history so the git scanner can infer branching
          # strategy and contributors.
          fetch-depth: 50

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8 # stable

      - name: Cache cargo artefacts
        uses: Swatinem/rust-cache@23869a5bd66c73db3c0ac40331f3206eb23791dc # v2.9.1
        with:
          shared-key: forge-analyze

      - name: Install forge
        run: cargo install --locked forge-dsl

      - name: Run forge analyze --merge
        run: |
          if [ ! -f architecture.forge ]; then
            echo "::error::architecture.forge not found. Bootstrap it first."
            exit 1
          fi
          forge analyze \
            --merge architecture.forge \
            --out architecture.forge \
            .

      - name: Check for changes
        id: diff
        run: |
          if git diff --quiet architecture.forge; then
            echo "changed=false" >> "$GITHUB_OUTPUT"
          else
            echo "changed=true" >> "$GITHUB_OUTPUT"
            git diff --stat architecture.forge
          fi

      - name: Open pull request
        if: steps.diff.outputs.changed == 'true'
        uses: peter-evans/create-pull-request@67ccf781d68cd99b580ae25a5c18a1cc84ffff1f # v7.0.6
        with:
          commit-message: "chore: refresh architecture.forge"
          title: "Refresh architecture.forge"
          body: |
            `forge analyze --merge` detected drift between the committed
            architecture model and the current codebase. Review and merge.
          branch: forge/refresh-architecture
          delete-branch: true
          labels: architecture, automated
```

## What happens on each push

1. Checkout and Rust toolchain set up.
2. `cargo install forge-dsl` — takes ~30 s cached, ~2 min cold.
3. `forge analyze --merge architecture.forge` — writes back to the
   same file.
4. `git diff --quiet` — if nothing changed, the job exits cleanly.
   If the model drifted, step 5 runs.
5. `peter-evans/create-pull-request` pushes a `forge/refresh-architecture`
   branch and opens or updates a PR.

If a refresh PR already exists, `create-pull-request` updates the
branch in place instead of opening a new PR, so you won't get a
stream of duplicate PRs.

## Adding the lint gate

Most teams run `forge check` as a separate job on every pull
request — not just the refresh ones. Add this to your existing
`ci.yml`:

```yaml
architecture-lint:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
    - uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8
    - run: cargo install --locked forge-dsl

    - name: Lint model
      run: forge check --source architecture.forge --format sarif > forge.sarif

    - name: Upload SARIF
      uses: github/codeql-action/upload-sarif@v3
      with:
        sarif_file: forge.sarif
```

Now every PR that touches the architecture model surfaces violations
as inline comments on the diff, and the PR cannot merge if any rule
is at `error` severity.

## Generate and publish alongside

If you also want a hosted HTML site that stays in sync with the
model, add a `pages` job to the same workflow:

```yaml
pages:
  needs: analyze
  runs-on: ubuntu-latest
  permissions:
    contents: read
    pages: write
    id-token: write
  environment:
    name: github-pages
    url: ${{ steps.deploy.outputs.page_url }}
  steps:
    - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
    - uses: dtolnay/rust-toolchain@29eef336d9b2848a0b548edc03f92a220660cdb8
    - run: cargo install --locked forge-dsl

    - name: Generate site
      run: |
        forge generate \
          --source architecture.forge \
          --base-url "/${{ github.event.repository.name }}/" \
          --out _site

    - uses: actions/configure-pages@v5
    - uses: actions/upload-pages-artifact@v3
      with:
        path: _site
    - id: deploy
      uses: actions/deploy-pages@v4
```

Now `https://yourorg.github.io/repo-name/` always reflects the
freshest merged model.

## Things that will bite you

- **The refresh PR will have merge conflicts if the model is also
  edited by hand on the same branch.** The fix is to rebase your
  hand-edits on top of the refresh PR, or merge the refresh first.
- **The first few runs will produce noisy diffs.** The scanners have
  heuristics; the very first time you wire CI up, expect to cull
  spurious `_inferred_*` containers. This settles down after two or
  three cycles.
- **`cargo install forge-dsl` is the slow step.** Cache it aggressively
  with `Swatinem/rust-cache`. Cold installs on a free runner are ~2
  minutes.
- **`merge` requires the existing file to parse.** If a hand-edit
  breaks the DSL syntax, the CI job fails loudly rather than
  overwriting the broken file. Fix the syntax and push again.

## See also

- [Merge mode](../user-guide/merge-mode.md) — exact merge semantics
- [Linting](../user-guide/linting.md) — all built-in rules and SARIF
- [Generating docs](../user-guide/generating-docs.md) — static site
  output
- [`forge/PUBLISHING.md`](../../forge/PUBLISHING.md) — more deployment
  recipes (Backstage TechDocs, S3, etc.)

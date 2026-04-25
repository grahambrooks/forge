# OpenAPI plugin (TypeScript example)

A reference Forge plugin that extracts Container metadata from `openapi.json`
and `swagger.json` files. Demonstrates the stdio protocol end-to-end: init
handshake → file dispatch → patch response.

## Run

```sh
# From the repo root, against any tree containing an openapi.json:
forge analyze ./path/to/project --plugin "npx tsx forge/examples/plugins/openapi/plugin.ts"
```

`tsx` runs the TypeScript directly with no compile step. The
`#!/usr/bin/env -S npx tsx` shebang means you can also `chmod +x plugin.ts`
and pass the path directly.

## What it does

For each matched file it emits one Container element with:

- `id`: `api.<slug-of-title>`
- `kind`: `Container`
- `technology`: `OpenAPI <version>`
- `tags`: `api:rest`
- `properties`: `openapi:title`, `openapi:version`, `openapi:routes`

Forge auto-stamps `inferred` and `inferred:plugin:openapi` tags, and a
`forge:source` property pointing to the spec file, so the result round-trips
through `forge analyze --merge` without trampling user-authored content.

## Limitations (deliberate, for the prototype)

- JSON only — no YAML parsing (would need an npm dep).
- No relationships emitted.
- No `finalize` round-trip.

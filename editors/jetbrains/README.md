# Forge Preview for JetBrains IDEs

Live diagram preview for `.forge` models in IntelliJ IDEA, RustRover, GoLand, WebStorm and the
other IntelliJ-based IDEs (2025.2 and later).

Open a `.forge` file and it appears in a split editor: source on the left, the rendered view on
the right. The preview toolbar has a picker for every view the model defines, a refresh button,
and zoom controls. You can also zoom with Ctrl/⌘ + mouse wheel.

- Re-renders 400 ms after you stop typing, including unsaved changes.
- Re-renders when any other `.forge` file changes, so edits to `!include`d files show up.
- The SVG palette follows the IDE theme, not the OS theme.
- A model error shows above the last good diagram, so a half-typed edit doesn't blank it.

## How it works

The plugin never parses the DSL. It runs the `forge` CLI:

1. `forge export --format json` to get the view keys and titles.
2. `forge build --out <temp dir>` to render each view as SVG.

The SVGs are shown in an embedded (JCEF) browser, so preview output is exactly what
`forge build` produces.

A saved file is rendered from disk. Unsaved edits go to the CLI through `--source -`, with
includes resolved from the file's directory. `--source -` needs forge 2026.9.8 or later; with an
older one the preview asks you to save or upgrade.

## Requirements

- The `forge` binary. The plugin looks in `PATH`, then `~/.cargo/bin`, `/opt/homebrew/bin` and
  `/usr/local/bin`. To use a different binary, set **Settings | Tools | Forge Preview**. The
  diagram style (outline or filled) is set there too.
- An IDE runtime with JCEF, which is the default for JetBrains Runtime.

Foundry (the Solidity toolchain) also installs a binary named `forge`. If yours comes first on
`PATH`, set the full path to Forge in settings.

For completion, diagnostics and hover, add `forge lsp` through LSP4IJ as described in
[EDITORS.md](../../forge/EDITORS.md#jetbrains-ides-intellij-webstorm-goland-etc). The two plugins
work together.

## Development

Needs JDK 21. Gradle picks it up through toolchains.

```bash
cd editors/jetbrains
./gradlew test          # unit tests, plus CLI tests against ../../forge/target/debug/forge
./gradlew buildPlugin   # → build/distributions/forge-preview-<version>.zip
./gradlew runIde -PopenFile=payments.forge   # sandbox IDE (the version it is compiled against)
./gradlew verifyPlugin  # JetBrains Plugin Verifier, against 2025.2 and 2026.2
```

The plugin is compiled against 2025.2 and loads in later releases too, so a newer IDE is only
exercised by running one. `runRustRover` starts a sandbox on a local install:

```bash
./gradlew runRustRover -PrustRoverPath=$HOME/Applications/RustRover.app/Contents
```

Do that after changing anything the platform has moved between releases. JCEF is the example:
it is part of the core up to 2025.x and a separate plugin from 2026.2, which the optional
`com.intellij.modules.jcef` dependency in `plugin.xml` covers. The Plugin Verifier does **not**
catch that kind of breakage — it reported the plugin as compatible with 2026.2 while the preview
failed to open there with `NoClassDefFoundError: JBCefApp`. Only running it catches that.

The CLI tests are skipped if the debug binary hasn't been built (in CI they fail instead). Run `make build` from the repo
root first.

To install a local build, choose **Settings | Plugins | ⚙ | Install Plugin from Disk…** and pick
the ZIP.

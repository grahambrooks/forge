# MkDocs Forge Plugin

Embed Forge architecture diagrams in your MkDocs documentation.

## Install

```bash
# Install the plugin
pip install ./integrations/mkdocs

# Ensure forge is on your PATH
cargo install --path forge
```

## Setup

Add to your `mkdocs.yml`:

```yaml
plugins:
  - forge:
      source: architecture.forge    # path relative to project root
      style: filled                 # or outline
      forge_bin: forge              # path to forge binary
```

## Usage

Reference views in any markdown file:

```markdown
# System Architecture

{{ forge_view "SystemContext" }}

## Container Details

{{ forge_view "Containers" }}

## CI/CD Pipeline

{{ forge_view "Pipeline" }}
```

The plugin replaces `{{ forge_view "Key" }}` with the rendered SVG inline. The SVGs are self-contained with embedded CSS and support light/dark mode.

## Live Reload

When using `mkdocs serve`, the plugin watches your `.forge` files for changes and rebuilds automatically.

## How It Works

1. On build, the plugin runs `forge build --source <file> --out <tmpdir>`
2. All generated SVGs are loaded into memory
3. During markdown processing, `{{ forge_view "Key" }}` patterns are replaced with the corresponding SVG content
4. SVGs are embedded inline (not as external images) so they:
   - Support dark/light mode via CSS
   - Scale responsively
   - Are searchable/accessible
   - Work in PDF exports

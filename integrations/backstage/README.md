# Backstage TechDocs Integration

This example shows how to publish Forge architecture documentation to Backstage TechDocs.

## Overview

There are two approaches:

1. **Pre-built HTML** — Forge generates a complete site, published directly to TechDocs storage
2. **MkDocs with Forge plugin** — Forge diagrams embedded in existing MkDocs documentation

This example uses approach 2 (MkDocs) since Backstage TechDocs natively supports MkDocs.

## Files

```
backstage/
├── catalog-info.yaml       # Backstage catalog entity
├── mkdocs.yml              # MkDocs config with Forge plugin
├── architecture.forge      # Architecture model
├── docs/
│   ├── index.md            # Landing page with system context
│   ├── containers.md       # Container details
│   └── pipeline.md         # CI/CD pipeline
└── .github/
    └── workflows/
        └── techdocs.yml    # CI workflow to publish
```

## Setup

### 1. Register in Backstage catalog

The `catalog-info.yaml` tells Backstage this is a TechDocs-enabled component:

```yaml
apiVersion: backstage.io/v1alpha1
kind: Component
metadata:
  name: payment-platform
  annotations:
    backstage.io/techdocs-ref: dir:.
```

### 2. Configure MkDocs with Forge

```yaml
# mkdocs.yml
site_name: Payment Platform Architecture
plugins:
  - techdocs-core
  - forge:
      source: architecture.forge
```

### 3. Write docs with embedded diagrams

```markdown
# System Architecture
{{ forge_view "SystemContext" }}
```

### 4. Publish via CI

The workflow builds the MkDocs site and publishes to your TechDocs storage (S3, GCS, or Azure Blob).

## Quick Start

```bash
# Install dependencies
pip install mkdocs-techdocs-core ./integrations/mkdocs
cargo install --path forge

# Preview locally
cd integrations/backstage
mkdocs serve

# Build for publishing
mkdocs build
```

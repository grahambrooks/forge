# Enterprise-Scale Multi-Project Catalogs

For organizations managing hundreds or thousands of repositories, Forge provides **catalog generation** — a way to aggregate multiple `.forge` models into a single unified documentation site with incremental build support.

## Overview

A **catalog** is a collection of project models, each describing a different system, service, or repository. Instead of generating separate documentation sites for each project, you generate one catalog site that provides:

- **Unified navigation** across all projects
- **Central index page** with project cards, tags, and search
- **Incremental builds** that only regenerate changed projects
- **Individual project pages** with full architecture diagrams and documentation

## Use Cases

- **Microservices architectures** with dozens of services
- **Multi-repository organizations** needing centralized documentation
- **Large engineering teams** with distributed ownership
- **Compliance/audit** requiring a complete view of all systems

---

## Quick Start

### 1. Create a Catalog File

Create a `.catalog` file (e.g., `enterprise.catalog`) describing your projects:

```forge
catalog "Enterprise Architecture" {
  description "Complete architecture documentation across all systems"

  project "payments" {
    name "Payment Processing Platform"
    description "Card and bank payment processing with PCI compliance"
    source "./projects/payments/forge.forge"
    repository "github.com/enterprise/payments-platform"
    tags "core" "pci" "fintech"
  }

  project "catalog-service" {
    name "Product Catalog API"
    description "Product inventory and catalog management"
    source "./projects/catalog/forge.forge"
    repository "github.com/enterprise/catalog-api"
    tags "core" "inventory"
  }

  project "notifications" {
    name "Notification Service"
    description "Email, SMS, and push notification delivery"
    source "./projects/notifications/forge.forge"
    repository "github.com/enterprise/notifications"
    tags "infrastructure" "messaging"
  }
}
```

### 2. Generate the Catalog Site

```bash
forge generate-catalog --source enterprise.catalog --out _site
```

Output:
```
Generating catalog site from "Enterprise Architecture"...
  3 projects
  Incremental mode: skipping unchanged projects
  Processed: 3 projects (0 skipped)
  Generated: 127 pages, 42 diagrams → _site
Done.
```

### 3. View the Result

The generated site has the following structure:

```
_site/
├── index.html               # Catalog index with project cards
├── assets/
│   └── forge.css           # Shared stylesheet
└── projects/
    ├── payments/
    │   ├── index.html      # Payment project homepage
    │   ├── views/          # Architecture views
    │   ├── elements/       # Element details
    │   ├── docs/           # Markdown documentation
    │   └── forge.json      # Machine-readable model
    ├── catalog-service/
    │   └── ...
    └── notifications/
        └── ...
```

---

## Catalog DSL Syntax

### Top-Level Block

```forge
catalog "<Name>" {
  description "<Overview of the catalog>"

  // Project definitions...
}
```

### Project Block

Each `project` block defines a single model to include in the catalog:

```forge
project "<key>" {
  name "<Display Name>"                    // Optional: defaults to key
  description "<Brief description>"        // Optional
  source "<path/to/model.forge>"          // Required: path to .forge file
  repository "<git-url>"                   // Optional: repository URL
  tags "<tag1>" "<tag2>" "<tag3>"         // Optional: project tags
}
```

**Fields:**
- `key` (string): Unique identifier for the project, used in URLs
- `name` (string): Human-readable display name
- `description` (string): Brief description shown on the catalog index
- `source` (string, required): Relative or absolute path to the `.forge` model file
- `repository` (string): Git repository URL for linking
- `tags` (list): Tags for categorization/filtering

---

## Incremental Builds

By default, `forge generate-catalog` uses **incremental mode**: only projects whose source files have changed are regenerated.

### How It Works

1. After generating a project, Forge stores the source file's modification time in `projects/<key>/.forge-meta`
2. On subsequent builds, Forge compares the current modification time against the stored value
3. If unchanged, the project is skipped
4. If changed (or if output doesn't exist), the project is regenerated

### Force Full Rebuild

To regenerate all projects regardless of modification time:

```bash
forge generate-catalog --source catalog.forge --no-incremental
```

### Performance Benefits

For a catalog with 100 projects:
- **Full build**: ~2-3 minutes (all 100 projects)
- **Incremental build** (1 changed): ~3-5 seconds (1 project + index)

---

## CLI Reference

### `forge generate-catalog`

Generate a multi-project catalog documentation website.

**Usage:**
```bash
forge generate-catalog [OPTIONS] --source <FILE>
```

**Options:**
- `--source <FILE>` — Input `.catalog` file (default: `forge.catalog`)
- `--out <DIR>` — Output directory (default: `_site`)
- `--title <TEXT>` — Site title (default: catalog name from file)
- `--base-url <URL>` — Base URL for deployment (default: `/`)
- `--style <STYLE>` — Diagram style: `filled` or `outline` (default: `outline`)
- `--no-incremental` — Disable incremental builds; regenerate all projects

**Examples:**

```bash
# Basic catalog generation
forge generate-catalog --source enterprise.catalog

# Custom output directory
forge generate-catalog --source catalog.forge --out docs/_site

# Deploy to subdirectory (e.g., GitHub Pages repo site)
forge generate-catalog --source enterprise.catalog --base-url /docs/

# Force full rebuild
forge generate-catalog --source catalog.forge --no-incremental
```

---

## Catalog Index Styling

The catalog index page uses semantic CSS classes for customization:

```css
.forge-catalog-index       /* Main catalog container */
.forge-header              /* Catalog title and description */
.forge-description         /* Catalog description text */
.forge-project-grid        /* Grid layout for project cards */
.forge-project-card        /* Individual project card */
.forge-project-repo        /* Repository URL display */
.forge-tags                /* Tag container */
.forge-tag                 /* Individual tag badge */
```

You can override these styles by including a custom CSS file after `forge.css`.

---

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Generate Architecture Docs

on:
  push:
    branches: [main]
    paths:
      - 'projects/**/*.forge'
      - 'enterprise.catalog'

jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Forge
        run: |
          curl -L https://github.com/your-org/forge/releases/latest/download/forge-linux-x86_64 -o forge
          chmod +x forge
          sudo mv forge /usr/local/bin/

      - name: Generate Catalog
        run: forge generate-catalog --source enterprise.catalog --out _site

      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./_site
```

### GitLab CI Example

```yaml
pages:
  stage: deploy
  image: rust:latest
  before_script:
    - cargo install forge-dsl
  script:
    - forge generate-catalog --source enterprise.catalog --out public
  artifacts:
    paths:
      - public
  only:
    - main
```

---

## Best Practices

### 1. **Organize Projects by Domain**

Group related projects with consistent tagging:

```forge
catalog "Engineering Platforms" {
  description "Internal platform services"

  project "auth" {
    name "Authentication Service"
    source "./auth/forge.forge"
    tags "platform" "security"
  }

  project "logging" {
    name "Centralized Logging"
    source "./logging/forge.forge"
    tags "platform" "observability"
  }

  project "metrics" {
    name "Metrics Collection"
    source "./metrics/forge.forge"
    tags "platform" "observability"
  }
}
```

### 2. **Use Relative Paths for Portability**

Keep catalog files and models in a single repository:

```
enterprise-architecture/
├── enterprise.catalog
└── projects/
    ├── payments/
    │   └── forge.forge
    ├── catalog/
    │   └── forge.forge
    └── notifications/
        └── forge.forge
```

Reference models with relative paths:

```forge
project "payments" {
  source "./projects/payments/forge.forge"
}
```

### 3. **Maintain a Single Source of Truth**

Don't duplicate `.forge` files. Use `!include` directives within individual models to compose from shared fragments:

```forge
// projects/payments/forge.forge
forge "Payment Platform" {
  !include ../shared/common-infra.forge
  !include ../shared/security-zones.forge

  model {
    // Payment-specific elements...
  }
}
```

### 4. **Tag for Discoverability**

Use consistent tags across projects:

```forge
tags "core"              // Business-critical systems
tags "platform"          // Infrastructure/platform services
tags "data"              // Data pipelines/storage
tags "pci"               // PCI DSL compliance required
tags "deprecated"        // Scheduled for retirement
```

### 5. **Automate Updates**

Set up CI/CD to regenerate the catalog on every push. With incremental builds, this is fast even for large catalogs.

---

## Comparison with Single-Project Generation

| Feature | `forge generate` | `forge generate-catalog` |
|---------|------------------|--------------------------|
| **Input** | One `.forge` file | One `.catalog` file referencing multiple `.forge` files |
| **Output** | Single documentation site | Multi-project site with unified index |
| **Navigation** | Per-model views and elements | Catalog index + per-project sites |
| **Incremental Builds** | N/A (always full rebuild) | Yes (skips unchanged projects) |
| **Use Case** | Single repository or monolith | Multiple repositories or microservices |
| **Build Time** | Seconds | Minutes (full), seconds (incremental) |

---

## Troubleshooting

### "project missing required 'source' field"

Every project block must have a `source` field pointing to a valid `.forge` file:

```forge
project "my-service" {
  name "My Service"
  source "./path/to/service.forge"  // Required!
}
```

### Projects not being skipped (slow incremental builds)

Check that:
1. Project source files aren't being touched/regenerated on every build
2. The `.forge-meta` files aren't being deleted (add to `.gitignore` if committing output)
3. File system modification times are preserved (some CI systems reset mtimes)

### Generated site has broken links

Ensure `--base-url` matches your deployment path:

```bash
# GitHub Pages project site at https://org.github.io/repo/
forge generate-catalog --source catalog.forge --base-url /repo/

# GitHub Pages user site at https://org.github.io/
forge generate-catalog --source catalog.forge --base-url /

# Custom domain
forge generate-catalog --source catalog.forge --base-url /
```

---

## Future Enhancements

Planned features for future releases:

- **Search index** across all projects (client-side search)
- **Cross-project relationships** (linking elements from different projects)
- **Baseline diffs** per project (highlight changes vs. previous version)
- **Federation** (merging catalogs from multiple sources)
- **Dependency graphs** (visualize inter-project dependencies)

---

## Related Documentation

- [DESIGN.md](./DESIGN.md) — Full Forge design specification
- [README.md](./README.md) — Getting started guide
- [PUBLISHING.md](./forge/PUBLISHING.md) — Deployment strategies

For questions or feedback, please open an issue at [github.com/your-org/forge/issues](https://github.com/your-org/forge/issues).

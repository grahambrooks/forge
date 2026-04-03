# Publishing Forge Sites

Forge generates static HTML sites that can be deployed anywhere static files are served. This guide covers the most common publishing targets.

## Quick Start

```bash
# Generate the site
forge generate --source architecture.forge --out _site

# Preview locally
cd _site && python3 -m http.server 8000
# Open http://localhost:8000
```

---

## GitHub Pages

### Option 1: GitHub Actions (recommended)

Create `.github/workflows/forge-docs.yml`:

```yaml
name: Publish Architecture Docs

on:
  push:
    branches: [main]
    paths:
      - '**.forge'
      - 'docs/**'

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Forge
        run: |
          curl -L https://github.com/your-org/forge/releases/latest/download/forge-linux-amd64 -o /usr/local/bin/forge
          chmod +x /usr/local/bin/forge
        # Or build from source:
        # - uses: dtolnay/rust-toolchain@stable
        # - run: cargo install --path forge

      - name: Generate site
        run: forge generate --source architecture.forge --out _site --base-url /repo-name/

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: _site

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

Then enable Pages in your repo settings:
1. Go to **Settings > Pages**
2. Set **Source** to **GitHub Actions**

The site will be available at `https://your-org.github.io/repo-name/`.

> **Important:** Set `--base-url /repo-name/` to match your repository name, otherwise CSS and navigation links will break.

### Option 2: Deploy to `gh-pages` branch

```bash
# Generate with the correct base URL
forge generate --source architecture.forge --out _site --base-url /repo-name/

# Deploy using ghp-import or manually
pip install ghp-import
ghp-import -n -p -f _site
```

Or with a simple script:

```bash
#!/bin/bash
set -e
REPO_NAME=$(basename $(git remote get-url origin) .git)
forge generate --source architecture.forge --out _site --base-url /${REPO_NAME}/
cd _site
git init
git checkout -b gh-pages
git add -A
git commit -m "Deploy architecture docs"
git push -f git@github.com:your-org/${REPO_NAME}.git gh-pages
```

### With architectural diff

To highlight recent changes in the published site:

```yaml
      - name: Generate site with diff
        run: |
          # Get the baseline from the previous release tag
          git show v1.0.0:architecture.forge > /tmp/baseline.forge 2>/dev/null || true
          if [ -f /tmp/baseline.forge ]; then
            forge generate --source architecture.forge --baseline /tmp/baseline.forge --out _site --base-url /repo-name/
          else
            forge generate --source architecture.forge --out _site --base-url /repo-name/
          fi
```

---

## Backstage TechDocs

[Backstage TechDocs](https://backstage.io/docs/features/techdocs/) serves documentation as part of your Backstage developer portal. Forge sites integrate as a TechDocs project.

### Setup

1. **Add `mkdocs.yml`** to your repo root (TechDocs uses MkDocs as its renderer, but we pre-build the HTML):

```yaml
site_name: Architecture Documentation
docs_dir: _site
nav:
  - Home: index.html
```

2. **Add TechDocs annotations** to your `catalog-info.yaml`:

```yaml
apiVersion: backstage.io/v1alpha1
kind: Component
metadata:
  name: payment-platform
  description: Payment processing architecture
  annotations:
    backstage.io/techdocs-ref: dir:.
spec:
  type: documentation
  lifecycle: production
  owner: platform-team
```

3. **Configure TechDocs to use pre-built HTML** in your Backstage `app-config.yaml`:

```yaml
techdocs:
  builder: 'external'
  publisher:
    type: 'awsS3'  # or 'googleGcs', 'azureBlobStorage'
    awsS3:
      bucketName: your-techdocs-bucket
      region: us-east-1
```

4. **Publish the pre-built site** in your CI pipeline:

```yaml
      - name: Generate Forge site
        run: forge generate --source architecture.forge --out _site

      - name: Publish to TechDocs
        run: |
          npx @techdocs/cli publish \
            --publisher-type awsS3 \
            --storage-name your-techdocs-bucket \
            --entity default/component/payment-platform \
            --directory _site
```

### Alternative: Embedded in existing MkDocs site

If your project already has MkDocs documentation, embed Forge diagrams directly:

```bash
# Generate SVGs into your docs directory
forge build --source architecture.forge --out docs/architecture/diagrams

# Generate JSON model for custom pages
forge generate --source architecture.forge --out docs/architecture
```

Then reference the SVGs in your markdown:

```markdown
# System Architecture

![System Context](diagrams/SystemContext.svg)

## Containers

![Containers](diagrams/Containers.svg)
```

### Backstage + GitHub Actions (complete example)

```yaml
name: TechDocs

on:
  push:
    branches: [main]
    paths:
      - '**.forge'
      - 'docs/**'
      - 'catalog-info.yaml'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Forge
        run: |
          curl -L https://github.com/your-org/forge/releases/latest/download/forge-linux-amd64 -o /usr/local/bin/forge
          chmod +x /usr/local/bin/forge

      - name: Generate architecture site
        run: forge generate --source architecture.forge --out _site

      - name: Publish to TechDocs
        env:
          AWS_ACCESS_KEY_ID: ${{ secrets.TECHDOCS_AWS_KEY }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.TECHDOCS_AWS_SECRET }}
        run: |
          npx @techdocs/cli publish \
            --publisher-type awsS3 \
            --storage-name ${{ vars.TECHDOCS_BUCKET }} \
            --entity default/component/${{ github.event.repository.name }} \
            --directory _site
```

---

## Other Static Hosts

### Netlify

Add `netlify.toml`:

```toml
[build]
  command = "forge generate --source architecture.forge --out _site"
  publish = "_site"
```

### Vercel

Add `vercel.json`:

```json
{
  "buildCommand": "forge generate --source architecture.forge --out _site",
  "outputDirectory": "_site"
}
```

### AWS S3 + CloudFront

```bash
forge generate --source architecture.forge --out _site
aws s3 sync _site s3://your-bucket/ --delete
aws cloudfront create-invalidation --distribution-id XXXXX --paths "/*"
```

### Docker (self-hosted)

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM nginx:alpine
COPY --from=builder /app/target/release/forge /usr/local/bin/forge
COPY architecture.forge /data/
RUN forge generate --source /data/architecture.forge --out /usr/share/nginx/html
EXPOSE 80
```

---

## CI Integration: Architecture Checks

Run `forge check` as a CI gate alongside publishing:

```yaml
      - name: Check architecture
        run: |
          forge check --source architecture.forge --severity warning --format sarif > results.sarif
          # Upload SARIF to GitHub Code Scanning (optional)
          # uses: github/codeql-action/upload-sarif@v3
          # with:
          #   sarif_file: results.sarif
```

This ensures architectural violations are caught before the site is published.

---

## Makefile Integration

Add to your project's Makefile:

```makefile
# Architecture documentation
docs: ## Generate architecture documentation site
	forge generate --source architecture.forge --out _site

docs-diff: ## Generate with diff highlighting against main
	git show main:architecture.forge > /tmp/baseline.forge 2>/dev/null || true
	@if [ -f /tmp/baseline.forge ]; then \
		forge generate --source architecture.forge --baseline /tmp/baseline.forge --out _site; \
	else \
		forge generate --source architecture.forge --out _site; \
	fi

docs-check: ## Run architecture checks
	forge check --source architecture.forge --severity warning

docs-publish: docs ## Publish to GitHub Pages
	ghp-import -n -p -f _site
```

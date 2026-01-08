# Release Please Integration Guide

> **Purpose**: Standalone instructions for integrating release-please into a repository.  
> **Audience**: AI agents or developers implementing automated releases.

---

## Overview

**release-please** is Google's tool for automating:
- CHANGELOG generation
- Version bumping in language-specific files
- GitHub Release creation
- Git tagging

It works by parsing **conventional commit messages** and creating "Release PRs" that, when merged, trigger the release process.

**It does NOT handle publication** (npm publish, cargo publish, pip upload) — those require separate workflow steps.

---

## Prerequisites

Before starting, verify:

1. **Conventional commits are used** (or will be adopted)
   - `feat:` = minor version bump
   - `fix:` = patch version bump  
   - `feat!:` or `fix!:` or `BREAKING CHANGE:` = major version bump
   - `chore:`, `docs:`, `style:`, `refactor:`, `test:` = no version bump

2. **Repository is on GitHub** (release-please is GitHub-native)

3. **You know the package structure**:
   - Single package or monorepo?
   - What languages? (Rust, Node, Python, Go, etc.)
   - Where are the packages located?

---

## Step 1: Identify Release Types

Map each package to a release-please strategy:

| Language | Release Type | Files Updated |
|----------|--------------|---------------|
| Rust | `rust` | `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` |
| Node.js/TypeScript | `node` | `package.json`, `package-lock.json`, `CHANGELOG.md` |
| Python | `python` | `pyproject.toml`, `setup.py`, `setup.cfg`, `*/__init__.py`, `CHANGELOG.md` |
| Go | `go` | `CHANGELOG.md` (version in git tags) |
| Java/Maven | `maven` | `pom.xml`, `CHANGELOG.md` |
| Simple/Generic | `simple` | `version.txt`, `CHANGELOG.md` |
| Helm | `helm` | `Chart.yaml`, `CHANGELOG.md` |
| Terraform | `terraform-module` | `README.md`, `CHANGELOG.md` |

---

## Step 2: Create Configuration Files

### For Single-Package Repositories

Create `.release-please-manifest.json` in repository root:

```json
{
  ".": "0.1.0"
}
```

Replace `"0.1.0"` with the current version of your package.

Create `release-please-config.json` in repository root:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "node",
  "bump-minor-pre-major": true,
  "bump-patch-for-minor-pre-major": true,
  "packages": {
    ".": {}
  }
}
```

Replace `"node"` with your release type from Step 1.

---

### For Monorepos

Create `.release-please-manifest.json`:

```json
{
  "packages/cli": "0.1.0",
  "packages/core": "0.1.0",
  "packages/sdk": "0.1.0"
}
```

Each key is the path to a package, value is its current version.

Create `release-please-config.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "bump-minor-pre-major": true,
  "bump-patch-for-minor-pre-major": true,
  "include-component-in-tag": true,
  "packages": {
    "packages/cli": {
      "release-type": "rust",
      "component": "cli"
    },
    "packages/core": {
      "release-type": "rust",
      "component": "core"
    },
    "packages/sdk": {
      "release-type": "node",
      "component": "sdk"
    }
  }
}
```

---

### For Rust Workspaces (Cargo)

Add the `cargo-workspace` plugin to handle workspace dependencies:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "bump-minor-pre-major": true,
  "include-component-in-tag": true,
  "packages": {
    "crates/cli": {
      "release-type": "rust",
      "component": "cli"
    },
    "crates/core": {
      "release-type": "rust",
      "component": "core"
    }
  },
  "plugins": [
    {
      "type": "cargo-workspace",
      "merge": false
    }
  ]
}
```

---

### For Node.js Workspaces (npm/yarn/pnpm)

Add the `node-workspace` plugin:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "bump-minor-pre-major": true,
  "include-component-in-tag": true,
  "packages": {
    "packages/core": {
      "release-type": "node",
      "component": "core"
    },
    "packages/cli": {
      "release-type": "node",
      "component": "cli"
    }
  },
  "plugins": [
    {
      "type": "node-workspace"
    }
  ]
}
```

---

## Step 3: Create GitHub Actions Workflow

Create `.github/workflows/release-please.yml`:

### Basic Workflow (Single Package)

```yaml
name: Release Please

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json
```

---

### Workflow with Publishing (Node.js)

```yaml
name: Release Please

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      release_created: ${{ steps.release.outputs.release_created }}
      tag_name: ${{ steps.release.outputs.tag_name }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json

  publish-npm:
    needs: release-please
    if: ${{ needs.release-please.outputs.release_created == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'
      - run: npm ci
      - run: npm publish
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

### Workflow with Publishing (Rust)

```yaml
name: Release Please

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      release_created: ${{ steps.release.outputs.release_created }}
      tag_name: ${{ steps.release.outputs.tag_name }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json

  publish-crates:
    needs: release-please
    if: ${{ needs.release-please.outputs.release_created == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish --token ${{ secrets.CRATES_IO_TOKEN }}
```

---

### Workflow with Publishing (Monorepo - Multiple Languages)

```yaml
name: Release Please

on:
  push:
    branches:
      - main

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      releases_created: ${{ steps.release.outputs.releases_created }}
      paths_released: ${{ steps.release.outputs.paths_released }}
      # Individual package outputs
      cli--release_created: ${{ steps.release.outputs['packages/cli--release_created'] }}
      sdk--release_created: ${{ steps.release.outputs['packages/sdk--release_created'] }}
    steps:
      - uses: googleapis/release-please-action@v4
        id: release
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json

  publish-cli:
    needs: release-please
    if: ${{ needs.release-please.outputs['cli--release_created'] == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish --token ${{ secrets.CRATES_IO_TOKEN }}
        working-directory: packages/cli

  publish-sdk:
    needs: release-please
    if: ${{ needs.release-please.outputs['sdk--release_created'] == 'true' }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          registry-url: 'https://registry.npmjs.org'
      - run: npm ci
        working-directory: packages/sdk
      - run: npm publish
        working-directory: packages/sdk
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

## Step 4: Bootstrap Existing Repository

If the repository has existing releases/tags, you need to bootstrap release-please.

### Option A: Start Fresh (Recommended for new projects)

Just create the config files with version `"0.1.0"` or your desired starting version.

### Option B: Bootstrap from Existing Tags

Run locally (requires Node.js):

```bash
npx release-please bootstrap \
  --token=$GITHUB_TOKEN \
  --repo-url=https://github.com/OWNER/REPO \
  --release-type=node
```

For monorepos, run with manifest mode:

```bash
npx release-please bootstrap \
  --token=$GITHUB_TOKEN \
  --repo-url=https://github.com/OWNER/REPO \
  --config-file=release-please-config.json \
  --manifest-file=.release-please-manifest.json
```

### Option C: Manual Bootstrap

1. Find the latest version tag for each package
2. Create `.release-please-manifest.json` with those versions
3. Ensure the manifest versions match the actual released versions

---

## Step 5: Configuration Options Reference

### Common Config Options

```json
{
  // Schema for validation
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  
  // Version bump behavior for pre-1.0 versions
  "bump-minor-pre-major": true,        // feat: bumps minor (not major) when < 1.0.0
  "bump-patch-for-minor-pre-major": true, // feat: bumps patch when < 1.0.0
  
  // Tagging
  "include-component-in-tag": true,    // Tags: "component-v1.0.0" instead of "v1.0.0"
  "tag-separator": "-",                // Separator between component and version
  
  // Pull Request
  "separate-pull-requests": false,     // One PR per package (true) or combined (false)
  "sequential-calls": false,           // Process packages sequentially
  
  // Changelog
  "changelog-type": "default",         // "default" or "github"
  "changelog-path": "CHANGELOG.md",    // Path to changelog file
  
  // Labels
  "release-label": "autorelease: pending",
  
  // Package definitions
  "packages": {
    "path/to/package": {
      "release-type": "node",
      "component": "package-name",     // Used in tags and PR titles
      "changelog-path": "CHANGELOG.md",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true
    }
  },
  
  // Plugins for workspace management
  "plugins": []
}
```

### Available Plugins

```json
{
  "plugins": [
    // For Cargo workspaces
    {
      "type": "cargo-workspace",
      "merge": false  // Don't merge package changelogs
    },
    
    // For Node.js workspaces
    {
      "type": "node-workspace"
    },
    
    // For Maven multi-module projects
    {
      "type": "maven-workspace"
    },
    
    // Link package versions together
    {
      "type": "linked-versions",
      "groupName": "my-packages",
      "components": ["package-a", "package-b"]
    },
    
    // Run command after version update
    {
      "type": "sentence-case"  // Sentence-case PR titles
    }
  ]
}
```

---

## Step 6: Verify Integration

### Checklist

- [ ] `release-please-config.json` exists and is valid JSON
- [ ] `.release-please-manifest.json` exists with correct package paths and versions
- [ ] `.github/workflows/release-please.yml` exists
- [ ] Workflow has correct permissions (`contents: write`, `pull-requests: write`)
- [ ] Branch name in workflow matches your default branch (`main` or `master`)
- [ ] For publishing: secrets are configured (`NPM_TOKEN`, `CRATES_IO_TOKEN`, etc.)

### Test the Integration

1. **Make a conventional commit and push to main**:
   ```bash
   git commit --allow-empty -m "feat: test release-please integration"
   git push origin main
   ```

2. **Check GitHub Actions** — the workflow should run

3. **Check for Release PR** — release-please should create a PR titled something like:
   - `chore(main): release 0.2.0` (single package)
   - `chore(main): release cli 0.2.0, sdk 0.1.1` (monorepo)

4. **Inspect the Release PR**:
   - Version bumps in package files (`package.json`, `Cargo.toml`, etc.)
   - CHANGELOG.md updates
   - Correct version numbers

5. **Merge the Release PR** — this triggers:
   - Git tag creation
   - GitHub Release creation
   - Publishing jobs (if configured)

---

## Troubleshooting

### Release PR Not Created

**Symptoms**: Workflow runs but no PR appears.

**Causes & Solutions**:

1. **No releasable commits since last release**
   - Only `feat:`, `fix:`, `deps:` trigger releases
   - `chore:`, `docs:`, `style:` do NOT trigger releases
   - Solution: Make a commit with releasable prefix

2. **Existing Release PR with `autorelease: pending` label**
   - release-please won't create new PR if one exists
   - Solution: Find and merge/close the existing PR, or remove the label

3. **Manifest version doesn't match tags**
   - Solution: Update `.release-please-manifest.json` to match latest released version

4. **Wrong branch configured**
   - Solution: Check workflow trigger branch matches your default branch

### "release_created" Output Always False

**Symptoms**: Publishing jobs never run.

**Causes & Solutions**:

1. **Release PR not merged** — outputs only become true after merge
2. **Output name mismatch in monorepo** — use correct path format:
   ```yaml
   # Correct format for monorepo outputs
   needs.release-please.outputs['packages/cli--release_created']
   ```

### Version Not Bumped Correctly

**Symptoms**: Wrong version number in Release PR.

**Causes & Solutions**:

1. **Manifest out of sync** — update `.release-please-manifest.json`
2. **Wrong release-type** — verify release-type matches your package manager
3. **Pre-1.0 behavior** — check `bump-minor-pre-major` setting

### CHANGELOG Not Updated

**Symptoms**: CHANGELOG.md unchanged or missing.

**Causes & Solutions**:

1. **File doesn't exist** — release-please creates it, but verify path
2. **Wrong changelog-path** — check config matches actual file location
3. **Different file name** — some projects use `HISTORY.md`, configure accordingly

---

## Examples

### Minimal Single Package (Node.js)

**Files to create:**

`.release-please-manifest.json`:
```json
{
  ".": "1.0.0"
}
```

`release-please-config.json`:
```json
{
  "packages": {
    ".": {
      "release-type": "node"
    }
  }
}
```

`.github/workflows/release-please.yml`:
```yaml
name: Release Please
on:
  push:
    branches: [main]
permissions:
  contents: write
  pull-requests: write
jobs:
  release-please:
    runs-on: ubuntu-latest
    steps:
      - uses: googleapis/release-please-action@v4
        with:
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json
```

---

### Rust Workspace Example

**Files to create:**

`.release-please-manifest.json`:
```json
{
  "crates/core": "0.1.0",
  "crates/cli": "0.1.0"
}
```

`release-please-config.json`:
```json
{
  "include-component-in-tag": true,
  "packages": {
    "crates/core": {
      "release-type": "rust",
      "component": "core"
    },
    "crates/cli": {
      "release-type": "rust",
      "component": "cli"
    }
  },
  "plugins": [
    {"type": "cargo-workspace"}
  ]
}
```

---

### Polyglot Monorepo Example (Rust + TypeScript + Python)

**Files to create:**

`.release-please-manifest.json`:
```json
{
  "packages/rust-core": "0.1.0",
  "packages/typescript-sdk": "0.1.0",
  "packages/python-sdk": "0.1.0"
}
```

`release-please-config.json`:
```json
{
  "include-component-in-tag": true,
  "separate-pull-requests": false,
  "packages": {
    "packages/rust-core": {
      "release-type": "rust",
      "component": "core"
    },
    "packages/typescript-sdk": {
      "release-type": "node",
      "component": "typescript-sdk"
    },
    "packages/python-sdk": {
      "release-type": "python",
      "component": "python-sdk"
    }
  }
}
```

---

## Quick Reference

### Conventional Commit → Version Bump

| Commit Prefix | Version Bump | Example |
|---------------|--------------|---------|
| `fix:` | Patch (0.0.X) | `fix: resolve null pointer` |
| `feat:` | Minor (0.X.0) | `feat: add new API endpoint` |
| `feat!:` | Major (X.0.0) | `feat!: redesign authentication` |
| `fix!:` | Major (X.0.0) | `fix!: change return type` |
| `BREAKING CHANGE:` in body | Major (X.0.0) | Any commit with this in body |
| `chore:` | None | `chore: update deps` |
| `docs:` | None | `docs: fix typo` |
| `refactor:` | None | `refactor: extract function` |

### Output Variables (for conditional publishing)

| Output | Description |
|--------|-------------|
| `release_created` | `'true'` if any release was created |
| `releases_created` | `'true'` if any releases were created (monorepo) |
| `tag_name` | Git tag name (e.g., `v1.2.3`) |
| `version` | Version number (e.g., `1.2.3`) |
| `paths_released` | JSON array of released package paths |
| `[path]--release_created` | Per-package release status (monorepo) |

---

## Files Summary

| File | Purpose | Required |
|------|---------|----------|
| `release-please-config.json` | Configuration for packages and plugins | Yes |
| `.release-please-manifest.json` | Tracks current versions | Yes |
| `.github/workflows/release-please.yml` | GitHub Actions workflow | Yes |
| `CHANGELOG.md` | Generated changelog (per package) | Auto-created |

---

## References

- [release-please GitHub](https://github.com/googleapis/release-please)
- [release-please-action](https://github.com/googleapis/release-please-action)
- [Configuration Schema](https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json)
- [Manifest Schema](https://raw.githubusercontent.com/googleapis/release-please/main/schemas/manifest.json)
- [Conventional Commits](https://www.conventionalcommits.org/)

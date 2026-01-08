# Testing Local Packages Before Publishing

This guide explains how to install and test packages from this monorepo in another local project before publishing to public registries.

## Overview

| Ecosystem | Most Realistic Method | Quick Dev Method |
|-----------|----------------------|------------------|
| Rust | Path dependency | Path dependency |
| Node/TypeScript | `npm pack` + install tarball | `npm link` |
| Python | `uv build` + install wheel | Editable install |

---

## Rust (Cargo)

### Path Dependencies

In your test project's `Cargo.toml`, reference the local package by path:

```toml
[dependencies]
hyperstack-sdk = { path = "/path/to/hyperstack-oss/main/rust/hyperstack-sdk" }
hyperstack = { path = "/path/to/hyperstack-oss/main/hyperstack" }
```

This is the standard method and behaves identically to a published crate.

### Validate Before Publishing

```bash
# Check what files would be included in the published crate
cargo package --list

# Simulate a full publish (builds and validates)
cargo publish --dry-run
```

---

## TypeScript / Node (npm, pnpm, yarn)

### Method 1: `npm link` (Symlink)

Creates a symlink between projects. Changes in the source are immediately reflected.

```bash
# Step 1: In this repo's typescript folder
cd typescript
npm link

# Step 2: In your test project
npm link <package-name>
```

To unlink:
```bash
npm unlink <package-name>
```

**Pros**: Instant updates during development  
**Cons**: Symlinks can behave differently than real installs (especially with peer dependencies)

### Method 2: `file:` Protocol

Add a file path directly in your test project's `package.json`:

```json
{
  "dependencies": {
    "your-package": "file:/path/to/hyperstack-oss/main/typescript"
  }
}
```

Then run `npm install`.

**Pros**: Simple, declarative  
**Cons**: Still uses symlinks under the hood

### Method 3: `npm pack` (Recommended for Realistic Testing)

Creates a `.tgz` tarball identical to what gets published to npm.

```bash
# Step 1: Create the tarball
cd typescript
npm pack
# Creates something like: your-package-1.0.0.tgz

# Step 2: Install in test project
cd /path/to/test-project
npm install /path/to/hyperstack-oss/main/typescript/your-package-1.0.0.tgz
```

**Pros**: Exact simulation of what users will download from npm  
**Cons**: Must re-pack after each change

### pnpm Equivalents

```bash
# Link
pnpm link /path/to/hyperstack-oss/main/typescript

# Pack and install
pnpm pack
pnpm add /path/to/your-package-1.0.0.tgz
```

### yarn Equivalents

```bash
# Link
cd typescript && yarn link
cd /test-project && yarn link <package-name>

# Pack and install
yarn pack
yarn add file:/path/to/your-package-1.0.0.tgz
```

---

## Python (uv, pip)

### Method 1: Editable Install

Installs the package in "development mode" - changes to source files are immediately available.

```bash
# With uv
uv pip install -e /path/to/hyperstack-oss/main/python/hyperstack-sdk

# With pip
pip install -e /path/to/hyperstack-oss/main/python/hyperstack-sdk
```

**Pros**: Instant updates during development  
**Cons**: Not representative of actual user install

### Method 2: Build and Install Wheel (Recommended for Realistic Testing)

Build a wheel (`.whl`) file and install it, exactly as users would from PyPI.

```bash
# With uv
cd python/hyperstack-sdk
uv build
uv pip install dist/hyperstack_sdk-*.whl

# With pip
cd python/hyperstack-sdk
pip install build
python -m build
pip install dist/hyperstack_sdk-*.whl
```

**Pros**: Exact simulation of PyPI install  
**Cons**: Must rebuild after each change

### Method 3: Path Dependency in `pyproject.toml` (uv)

For projects using uv, you can declare path dependencies:

```toml
[project]
dependencies = [
    "hyperstack-sdk",
]

[tool.uv.sources]
hyperstack-sdk = { path = "/path/to/hyperstack-oss/main/python/hyperstack-sdk" }
```

Then run:
```bash
uv sync
```

To switch back to the published version, remove the `[tool.uv.sources]` entry.

### Validate Before Publishing

```bash
# Check package metadata and structure
uv build
twine check dist/*

# Test upload to TestPyPI first
twine upload --repository testpypi dist/*
```

---

## Tips

1. **Use realistic testing before releases**: `npm pack`, `uv build`, and `cargo publish --dry-run` catch issues that symlinks miss (missing files, incorrect exports, etc.)

2. **Automate with scripts**: Add npm/cargo/uv scripts to automate the pack-and-install workflow

3. **Test in clean environments**: Consider using Docker or virtual environments to ensure no implicit dependencies leak through

4. **Check what's included**: 
   - Rust: `cargo package --list`
   - Node: Check `files` field in `package.json` or use `npm pack --dry-run`
   - Python: Check `[tool.setuptools]` or `include`/`exclude` in `pyproject.toml`

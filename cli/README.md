# hyperstack-cli

[![crates.io](https://img.shields.io/crates/v/hyperstack-cli.svg)](https://crates.io/crates/hyperstack-cli)
[![docs.rs](https://docs.rs/hyperstack-cli/badge.svg)](https://docs.rs/hyperstack-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Command-line tool for building, deploying, and managing HyperStack stream specifications.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/HyperTekOrg/hyperstack.git
cd hyperstack

# Build and install the CLI
cargo install --path cli

# Or just build (binary will be at target/release/hs)
cargo build --release -p hyperstack-cli
```

## Quick Start

1. **Initialize configuration**

```bash
hs config init
```

This creates a `hyperstack.toml` file with example configurations.

2. **Authenticate**

```bash
# Register a new account
hs auth register

# Or login to an existing account
hs auth login
```

3. **Edit `hyperstack.toml`** to configure your specs:

```toml
[project]
name = "my-project"

[sdk]
output_dir = "./generated"
typescript_package = "hyperstack-react"  # optional
rust_crate_prefix = "hyperstack"         # optional, prefix for generated crate names

# Specs are auto-discovered from .hyperstack/*.ast.json
# Define explicitly for custom naming:
[[specs]]
name = "settlement-game"      # optional, defaults to kebab-case of entity
ast = "SettlementGame"        # entity name or path to .ast.json
description = "Settlement game state tracking"  # optional
```

4. **Push your spec to the cloud**

```bash
hs spec push settlement-game
```

5. **Build and deploy**

```bash
# Create a build from your spec
hs build create settlement-game

# Watch the build progress
hs build status <build-id> --watch
```

6. **Connect to your deployment**

```bash
# View deployment info including WebSocket URL
hs deploy info <build-id>
```

## Commands

### Authentication

#### `hs auth register`
Register a new account.

#### `hs auth login`
Login to your account.

#### `hs auth logout`
Logout and remove stored credentials.

#### `hs auth status`
Check if you're authenticated (local check only).

#### `hs auth whoami`
Verify your authentication with the server and show account info.

### Spec Management

#### `hs spec push [spec-name]`
Push local specs with their AST to remote. Reads specs from `hyperstack.toml` and uploads AST from `.hyperstack/<entity>.ast.json`.

```bash
# Push all specs
hs spec push

# Push specific spec
hs spec push settlement-game
```

#### `hs spec pull`
Pull remote specs to local config.

#### `hs spec list`
List all remote specs.

#### `hs spec versions <spec-name>`
Show version history for a spec.

```bash
hs spec versions settlement-game --limit 10
```

#### `hs spec show <spec-name>`
Show detailed spec information.

```bash
# Show spec info with latest version
hs spec show settlement-game

# Show specific version details
hs spec show settlement-game --version 3
```

#### `hs spec delete <spec-name>`
Delete a spec from remote.

```bash
# Interactive confirmation
hs spec delete settlement-game

# Skip confirmation
hs spec delete settlement-game --force
```

### Build Commands

#### `hs build create <spec-name>`
Create a new build from a spec.

```bash
# Build latest version
hs build create settlement-game

# Build specific version
hs build create settlement-game --version 2

# Build from local AST file
hs build create settlement-game --ast-file ./my-spec.ast.json
```

#### `hs build list`
List builds for your account.

```bash
# List builds
hs build list

# Filter by status
hs build list --status completed

# Limit results
hs build list --limit 10
```

#### `hs build status <build-id>`
Get detailed build status.

```bash
# Show status
hs build status 123

# Watch progress until completion
hs build status 123 --watch

# Output as JSON
hs build status 123 --json
```

#### `hs build logs <build-id>`
View build logs (opens CloudWatch logs URL).

```bash
hs build logs 123
```

### Deployment Commands

#### `hs deploy info <build-id>`
Show deployment info for a completed build.

```bash
hs deploy info 123
```

Output includes:
- Atom name
- Namespace
- WebSocket URL
- Container image

#### `hs deploy list`
List all active deployments.

```bash
hs deploy list --limit 20
```

### SDK Generation

#### `hs sdk list`
List all available specs from your configuration.

#### `hs sdk create typescript <spec-name>`
Generate a TypeScript SDK for the specified spec.

```bash
# Generate with default settings
hs sdk create typescript settlement-game

# Generate with custom output path
hs sdk create typescript settlement-game --output ./my-sdk/game.ts

# Generate with custom package name
hs sdk create typescript settlement-game --package-name @myorg/game-sdk
```

#### `hs sdk create rust <spec-name>`
Generate a Rust SDK crate for the specified spec.

```bash
# Generate with default settings (outputs to ./generated/<spec-name>-stack/)
hs sdk create rust settlement-game

# Generate with custom output directory
hs sdk create rust settlement-game --output ./crates/game-sdk

# Generate with custom crate name
hs sdk create rust settlement-game --crate-name game-sdk
```

The generated crate includes:
- `Cargo.toml` with hyperstack-sdk dependency
- `src/lib.rs` with re-exports
- `src/types.rs` with all data structs
- `src/entity.rs` with Entity trait implementations

Usage after generation:

```toml
# Add to your Cargo.toml
[dependencies]
hyperstack-sdk = "0.2"
settlement-game-stack = { path = "./generated/settlement-game-stack" }
```

```rust
use hyperstack_sdk::HyperStack;
use settlement_game_stack::{SettlementGame, SettlementGameEntity};

let hs = HyperStack::connect("wss://example.com").await?;
let game = hs.get::<SettlementGameEntity>("game_id").await;
```

### Configuration Management

#### `hs config init`
Initialize a new `hyperstack.toml` configuration file.

#### `hs config validate`
Validate your configuration file.

## Configuration File Format

The `hyperstack.toml` file defines your project and available specs. The CLI auto-discovers AST files, so minimal configuration is needed:

```toml
[project]
name = "my-hyperstack-project"  # required

[sdk]
output_dir = "./generated"              # optional, defaults to "./generated"
typescript_package = "hyperstack-react" # optional, npm package for generated SDK
rust_crate_prefix = "hyperstack"        # optional, prefix for generated Rust crate names

[build]
watch_by_default = true                 # optional, stream build progress

# Specs are auto-discovered from .hyperstack/*.ast.json files
# Only define [[specs]] if you need custom naming or explicit configuration
[[specs]]
name = "my-spec"                        # optional, defaults to kebab-case of entity
ast = "MyEntity"                        # required: entity name or path to .ast.json
description = "Optional description"    # optional
```

### Minimal Configuration

For most projects, you only need:

```toml
[project]
name = "my-project"
```

The CLI will auto-discover all `.hyperstack/*.ast.json` files and derive spec names from entity names (e.g., `SettlementGame` → `settlement-game`).

## Build Process

When you create a build, the following happens:

1. **AST Validation**: The spec's AST is validated
2. **Upload**: AST is uploaded to S3
3. **Queue**: Build is queued in CodeBuild
4. **Build**: Docker image is built from the AST
5. **Push**: Image is pushed to ECR
6. **Deploy**: Container is deployed to Kubernetes
7. **Complete**: WebSocket URL becomes available

Build statuses: `pending` -> `uploading` -> `queued` -> `building` -> `pushing` -> `deploying` -> `completed`/`failed`

## Environment Variables

- `HYPERSTACK_API_URL`: Override the API URL (default: `http://localhost:3000`)

## How It Works

The CLI uses **AST serialization** to generate SDKs and deployments efficiently:

1. **The `#[stream_spec]` macro** automatically serializes the AST to `.hyperstack/<entity_name>.ast.json` when you compile your spec
2. **CLI auto-discovers** AST files - no manual configuration required for simple projects
3. **Push to cloud**: The AST is uploaded and versioned
4. **Build**: CodeBuild compiles the AST into a deployable container
5. **Deploy**: The container runs as a WebSocket-accessible atom

This means:
- Zero-config for single-spec projects (just run `hyperstack up`)
- No need to recompile the CLI when adding new specs
- No feature flags or hardcoded registries
- Works with any spec in your workspace
- Content-addressed versioning (same AST = same version)
- Fast, efficient builds using pre-compiled base images

## Troubleshooting

### Error: "Not authenticated"
Run `hs auth login` to authenticate.

### Error: "Spec 'xxx' not found"
Make sure the AST file exists at `.hyperstack/<entity>.ast.json`. Run:
```bash
hs sdk list
```
This shows all auto-discovered and configured specs.

### Error: "AST file not found"
Run `cargo build` in your spec crate to generate the AST file at `.hyperstack/<entity>.ast.json`.

### Error: "Spec has no versions"
Push your spec first:
```bash
hs spec push <spec-name>
```

### Build failed
Check the build logs:
```bash
hs build logs <build-id>
```

## License

Apache-2.0

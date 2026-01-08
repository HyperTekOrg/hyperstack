# hyperstack-cli

[![crates.io](https://img.shields.io/crates/v/hyperstack-cli.svg)](https://crates.io/crates/hyperstack-cli)
[![docs.rs](https://docs.rs/hyperstack-cli/badge.svg)](https://docs.rs/hyperstack-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Command-line tool for building, deploying, and managing HyperStack stream specifications.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/HyperTekOrg/hyper-stack-platform
cd hyper-stack-platform

# Build and install the CLI
cargo install --path oss/packages/cli

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
output_dir = "./generated"

[[specs]]
name = "settlement-game"
entity_name = "SettlementGame"
crate_name = "flip-atom"
module_path = "flip_atom::spec::settlement_spec"
# Optional fields:
description = "Settlement game state tracking"
package_name = "@hyperstack/flip-sdk"
output_path = "./custom/path.ts"
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

### Configuration Management

#### `hs config init`
Initialize a new `hyperstack.toml` configuration file.

#### `hs config validate`
Validate your configuration file.

## Configuration File Format

The `hyperstack.toml` file defines your project and available specs:

```toml
[project]
# Project name (required)
name = "my-hyperstack-project"

# Default output directory for generated SDKs (optional, defaults to "./generated")
output_dir = "./generated"

# Define one or more specs
[[specs]]
# Required fields:
name = "spec-name"                     # Used in CLI commands
entity_name = "EntityName"             # Name of the entity in the spec
crate_name = "crate-name"              # Crate containing the spec
module_path = "crate::module::path"    # Full module path to spec

# Optional fields:
description = "Optional description"   # Human-readable description
package_name = "@org/package"          # TypeScript package name (defaults to @hyperstack/sdk)
output_path = "./custom/path.ts"       # Custom output path (overrides output_dir)
```

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

1. **You define specs** in `hyperstack.toml`
2. **The `#[stream_spec]` macro** automatically serializes the AST to `.hyperstack/<entity_name>.ast.json` when you compile your spec
3. **Push to cloud**: The AST is uploaded and versioned
4. **Build**: CodeBuild compiles the AST into a deployable container
5. **Deploy**: The container runs as a WebSocket-accessible atom

This means:
- No need to recompile the CLI when adding new specs
- No feature flags or hardcoded registries
- Works with any spec in your workspace
- Content-addressed versioning (same AST = same version)
- Fast, efficient builds using pre-compiled base images

## Troubleshooting

### Error: "Not authenticated"
Run `hs auth login` to authenticate.

### Error: "Spec 'xxx' not found"
Make sure the spec is defined in your `hyperstack.toml` file. Run:
```bash
hs sdk list
```

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

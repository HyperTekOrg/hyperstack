# hyperstack-cli

[![crates.io](https://img.shields.io/crates/v/hyperstack-cli.svg)](https://crates.io/crates/hyperstack-cli)
[![docs.rs](https://docs.rs/hyperstack-cli/badge.svg)](https://docs.rs/hyperstack-cli)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Command-line tool for building, deploying, and managing HyperStack stream stacks.

## Installation

```bash
cargo install hyperstack-cli
```

### From Source

```bash
git clone https://github.com/HyperTekOrg/hyperstack.git
cd hyperstack
cargo install --path cli
```

## Quick Start

```bash
# Initialize project (auto-discovers AST files)
hs init

# Authenticate
hs auth login

# Deploy
hs up
```

That's it! Your stack is deployed and you'll see the WebSocket URL.

## Command Overview

| Command | Description |
|---------|-------------|
| `hs init` | Initialize project |
| `hs up [stack]` | Deploy (push + build + deploy) |
| `hs status` | Show project overview |
| `hs stack list` | List all stacks |
| `hs stack show <name>` | Show stack details |
| `hs stack rollback <name>` | Rollback to previous version |

## Daily Workflow

```bash
# Make changes to your stack, rebuild
cargo build

# Deploy
hs up

# Check status
hs status
```

## Stack Commands

### `hs stack list`

List all stacks with deployment status:

```
STACK              STATUS     VERSION  URL
settlement-game    active     v3       wss://settlement-game.stack.usehyperstack.com
token-tracker      active     v1       wss://token-tracker.stack.usehyperstack.com
```

### `hs stack show <name>`

Show detailed information:

```bash
hs stack show settlement-game
```

Shows: entity info, deployment status, version history, recent builds.

### `hs stack push [name]`

Push local stacks to remote without deploying:

```bash
hs stack push                  # Push all
hs stack push settlement-game  # Push one
```

### `hs stack versions <name>`

Show version history:

```bash
hs stack versions settlement-game --limit 10
```

### `hs stack rollback <name>`

Rollback to a previous version:

```bash
hs stack rollback settlement-game          # Previous version
hs stack rollback settlement-game --to 2   # Specific version
```

### `hs stack delete <name>`

Delete a stack:

```bash
hs stack delete settlement-game
```

## Deployment

### `hs up [stack-name]`

The happy path - push, build, and deploy in one command:

```bash
hs up                              # Deploy all
hs up settlement-game              # Deploy one
hs up settlement-game --branch staging  # Branch deploy
hs up settlement-game --preview    # Preview deployment
```

## Authentication

```bash
hs auth register    # Create new account
hs auth login       # Login
hs auth logout      # Logout
hs auth whoami      # Verify with server
```

Credentials: `~/.hyperstack/credentials.toml`

## SDK Generation

```bash
hs sdk list                                   # List available stacks
hs sdk create typescript settlement-game      # Generate TypeScript SDK
hs sdk create rust settlement-game            # Generate Rust SDK
```

## Configuration

**File:** `hyperstack.toml`

```toml
[project]
name = "my-project"

[sdk]
output_dir = "./generated"

# Stacks - auto-discovered from .hyperstack/*.ast.json
# Define explicitly for custom naming:
[[stacks]]
name = "my-game"
ast = "SettlementGame"
```

For most projects, you only need:

```toml
[project]
name = "my-project"
```

The CLI auto-discovers stacks from `.hyperstack/*.ast.json` files.

## WebSocket URLs

| Type | Pattern |
|------|---------|
| Production | `wss://{stack-name}.stack.usehyperstack.com` |
| Branch | `wss://{stack-name}-{branch}.stack.usehyperstack.com` |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HYPERSTACK_API_URL` | Override API endpoint |

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Not authenticated` | Run `hs auth login` |
| `Stack not found` | Check `hs stack list` |
| `AST file not found` | Run `cargo build` to generate AST |
| `Build failed` | Check `hs status` for build details |

## License

Apache-2.0

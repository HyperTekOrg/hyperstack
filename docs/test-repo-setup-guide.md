# Test Repo Setup Guide for Local Hyperstack Development

This guide explains how to set up a test repository that uses the local hyperstack packages (Rust SDK, TypeScript SDK, and CLI) to deploy a stack and consume streaming data.

## Monorepo Package Reference

| Package | Location | Package Name | Registry | Purpose |
|---------|----------|--------------|----------|---------|
| Rust umbrella | `hyperstack/` | `hyperstack` | crates.io | Re-exports all Rust components |
| Rust SDK | `rust/hyperstack-sdk/` | `hyperstack-sdk` | crates.io | Rust client for WebSocket streams |
| CLI | `cli/` | `hyperstack-cli` | crates.io | Deploy specs, generate SDKs |
| TypeScript SDK | `typescript/` | `@hyperstack/react` | npm | React hooks for streaming data |
| Interpreter | `interpreter/` | `hyperstack-interpreter` | crates.io | AST transformation runtime |
| Server | `rust/hyperstack-server/` | `hyperstack-server` | crates.io | WebSocket server |
| Spec Macros | `spec-macros/` | `hyperstack-spec-macros` | crates.io | Proc-macros for specs |

## Directory Structure

```
hypertek/
├── hyperstack-oss/main/    # Source monorepo (this repo)
└── hyperstack-test/        # Your test repo (to be created)
    ├── Cargo.toml          # Rust project with path dependencies
    ├── src/
    │   └── main.rs         # Rust client example
    ├── hyperstack.toml     # Hyperstack CLI configuration
    └── frontend/           # TypeScript/React app
        ├── package.json
        └── src/
            ├── App.tsx
            └── generated/  # Generated TypeScript SDK output
```

---

## Step 1: Create Test Repo Directory

```bash
cd /Users/adrian/code/defi/hypertek
mkdir hyperstack-test && cd hyperstack-test
```

---

## Step 2: Initialize Rust Project

```bash
cargo init --name hyperstack-test
```

Replace `Cargo.toml` with:

```toml
[package]
name = "hyperstack-test"
version = "0.1.0"
edition = "2021"

[dependencies]
# Local path to umbrella crate (includes interpreter, spec-macros, server)
# Use "full" feature to include the SDK as well
hyperstack = { path = "../hyperstack-oss/main/hyperstack", features = ["full"] }

# Local Rust SDK for client usage (can also access via hyperstack with "full" feature)
hyperstack-sdk = { path = "../hyperstack-oss/main/rust/hyperstack-sdk" }

# Required dependencies
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

---

## Step 3: Install CLI from Local Source

```bash
# Build and install CLI globally from the monorepo
cargo install --path ../hyperstack-oss/main/cli

# Verify installation
hyperstack --version

# The binary is installed to ~/.cargo/bin/hyperstack
```

### CLI Commands Overview

| Command | Description |
|---------|-------------|
| `hyperstack init` | Initialize hyperstack.toml with auto-detected AST files |
| `hyperstack up [spec]` | Push, build, and deploy a spec (all-in-one) |
| `hyperstack status` | Show overview of specs, builds, deployments |
| `hyperstack auth login` | Authenticate with Hyperstack cloud |
| `hyperstack spec push` | Push spec AST to cloud |
| `hyperstack build create <spec>` | Create a build from a spec |
| `hyperstack build status <id> --watch` | Watch build progress |
| `hyperstack deploy info <id>` | Get deployment WebSocket URL |
| `hyperstack sdk create typescript <spec>` | Generate TypeScript SDK |

---

## Step 4: Set Up TypeScript/React Frontend

```bash
mkdir -p frontend && cd frontend
npm init -y
```

### Option A: npm link (Recommended for Development)

Live updates - changes in monorepo are immediately reflected:

```bash
# Step 1: Register the package globally (run once)
cd /Users/adrian/code/defi/hypertek/hyperstack-oss/main/typescript
npm link

# Step 2: Link in your test project
cd /Users/adrian/code/defi/hypertek/hyperstack-test/frontend
npm link @hyperstack/react
```

### Option B: file: Protocol

Simpler but uses symlinks under the hood:

```json
{
  "dependencies": {
    "@hyperstack/react": "file:../../hyperstack-oss/main/typescript"
  }
}
```

Then run `npm install`.

### Option C: npm pack (Most Realistic)

Creates a tarball identical to npm publish:

```bash
cd /Users/adrian/code/defi/hypertek/hyperstack-oss/main/typescript
npm pack
# Creates: hyperstack-react-0.1.0.tgz

cd /Users/adrian/code/defi/hypertek/hyperstack-test/frontend
npm install ../../hyperstack-oss/main/typescript/hyperstack-react-0.1.0.tgz
```

### Install Peer Dependencies

Whichever method you use, install the peer dependencies:

```bash
npm install react@^18 react-dom@^18 zustand@^4
npm install -D typescript @types/react @types/react-dom
```

---

## Step 5: Initialize Hyperstack Project Configuration

```bash
cd /Users/adrian/code/defi/hypertek/hyperstack-test
hyperstack init
```

This creates `hyperstack.toml`. Edit it for your spec:

```toml
[project]
name = "my-test-stack"
output_dir = "./generated"

[[specs]]
name = "my-stream"
entity_name = "MyEntity"
crate_name = "hyperstack-test"
module_path = "hyperstack_test::my_spec"
description = "Test stream specification"
package_name = "@hyperstack/my-sdk"
output_path = "./frontend/src/generated/my-sdk.ts"
```

### Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Spec identifier used in CLI commands |
| `entity_name` | Yes | Name of the entity in the spec (e.g., `SettlementGame`) |
| `crate_name` | Yes | Rust crate containing the spec |
| `module_path` | Yes | Full module path to spec (e.g., `my_crate::spec::my_spec`) |
| `description` | No | Human-readable description |
| `package_name` | No | TypeScript package name (default: `@hyperstack/sdk`) |
| `output_path` | No | Custom output path for generated SDK |

---

## Step 6: Authenticate and Deploy

```bash
# Login (or register if new account)
hyperstack auth login

# Deploy everything in one command (push + build + deploy)
hyperstack up my-stream

# Or step by step:
hyperstack spec push my-stream
hyperstack build create my-stream
hyperstack build status <build-id> --watch
hyperstack deploy info <build-id>
```

### Build Status Flow

```
pending → uploading → queued → building → pushing → deploying → completed
```

After completion, you'll get a WebSocket URL like:
```
wss://my-stream.stack.hypertek.app
```

---

## Step 7: Generate TypeScript SDK

```bash
hyperstack sdk create typescript my-stream --output ./frontend/src/generated/my-sdk.ts
```

This generates typed client code based on your spec's AST.

---

## Step 8: Use TypeScript SDK in React

Create `frontend/src/App.tsx`:

```tsx
import React from 'react';
import { HyperstackProvider, useHyperstack, defineStack } from '@hyperstack/react';

// Option 1: Use generated SDK (after running hyperstack sdk create)
// import { MyEntityStack } from './generated/my-sdk';

// Option 2: Define stack inline for quick testing
const MyStack = defineStack({
  name: 'my-stream',
  websocketUrl: 'wss://my-stream.stack.hypertek.app',
  views: {
    myEntity: {
      kv: { mode: 'kv', entity: 'MyEntity' },
      state: { mode: 'state', entity: 'MyEntity' },
      list: { mode: 'list', entity: 'MyEntity' },
    }
  }
});

function StreamConsumer() {
  const stack = useHyperstack(MyStack);
  
  // Subscribe to all entities (kv mode)
  const { data: entities, connectionState } = stack.views.myEntity.kv.use();
  
  if (connectionState === 'connecting') {
    return <div>Connecting...</div>;
  }
  
  if (connectionState === 'error') {
    return <div>Connection error</div>;
  }
  
  return (
    <div>
      <h1>Live Entities</h1>
      {entities && Array.from(entities.entries()).map(([key, entity]) => (
        <div key={key}>
          <strong>{key}:</strong> {JSON.stringify(entity)}
        </div>
      ))}
    </div>
  );
}

function App() {
  return (
    <HyperstackProvider config={{ 
      websocketUrl: 'wss://my-stream.stack.hypertek.app',
      autoSubscribeDefault: true 
    }}>
      <StreamConsumer />
    </HyperstackProvider>
  );
}

export default App;
```

### Available Hooks

| Hook | Usage | Returns |
|------|-------|---------|
| `useHyperstack(stack)` | Main hook for stack access | Stack instance with views |
| `useHyperState({ view, key? })` | Low-level state hook | `{ entity, entities, connectionState }` |
| `useEntity(view, key)` | Single entity by key | Entity data or undefined |
| `useEntities(view)` | All entities in view | `Map<string, Entity>` |

### View Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `kv` | Key-value map of all entities | Dashboard showing all items |
| `state` | Single entity by key | Detail view of one item |
| `list` | Array of entities with pagination | Paginated lists |

---

## Step 9: Use Rust SDK (Alternative to TypeScript)

Create `src/main.rs`:

```rust
use hyperstack_sdk::HyperStackClient;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

// Define your entity structure matching the spec
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct MyEntity {
    id: Option<String>,
    name: Option<String>,
    value: Option<i64>,
    // Add fields matching your spec
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // WebSocket URL from deployment
    let url = std::env::var("WS_URL")
        .unwrap_or_else(|_| "wss://my-stream.stack.hypertek.app".to_string());
    
    // View to subscribe to (EntityName/mode)
    let view = std::env::var("VIEW")
        .unwrap_or_else(|_| "MyEntity/kv".to_string());
    
    println!("Connecting to {} view {}...", url, view);
    
    let mut client = HyperStackClient::<MyEntity>::new(&url, &view);
    
    // Optional: filter to specific key
    if let Ok(key) = std::env::var("KEY") {
        client = client.with_key(key);
    }
    
    let store = client.connect().await?;
    println!("Connected! Watching for updates...\n");
    
    let mut updates = store.subscribe();
    
    loop {
        tokio::select! {
            Ok((key, entity)) = updates.recv() => {
                println!("=== Update: {} ===", key);
                println!("{}", serde_json::to_string_pretty(&entity)?);
                println!();
            }
            _ = sleep(Duration::from_secs(30)) => {
                println!("No updates for 30s, still listening...");
            }
        }
    }
}
```

### Run the Rust Client

```bash
# Watch all entities
cargo run

# Watch specific view
VIEW=MyEntity/list cargo run

# Watch single entity by key
VIEW=MyEntity/state KEY=entity123 cargo run

# Custom WebSocket URL
WS_URL=wss://custom.endpoint.com cargo run
```

---

## Complete File Checklist

After setup, your test repo should have:

```
hyperstack-test/
├── Cargo.toml              # Rust dependencies with path to local packages
├── Cargo.lock
├── hyperstack.toml         # Hyperstack CLI configuration
├── src/
│   └── main.rs             # Rust SDK client example
└── frontend/
    ├── package.json        # With @hyperstack/react dependency
    ├── package-lock.json
    ├── tsconfig.json
    └── src/
        ├── App.tsx         # React app using TypeScript SDK
        └── generated/
            └── my-sdk.ts   # Generated by hyperstack sdk create
```

---

## Troubleshooting

### "Not authenticated"
```bash
hyperstack auth login
```

### "Spec not found in hyperstack.toml"
Ensure the spec name in CLI command matches `[[specs]] name` in config.

### "AST file not found"
Run `cargo build` in your spec crate first. The `#[stream_spec]` macro generates `.hyperstack/<entity>.ast.json` during compilation.

### TypeScript SDK import errors
Ensure you've built the SDK before importing:
```bash
cd ../hyperstack-oss/main/typescript
npm run build
```

### WebSocket connection fails
1. Verify deployment completed: `hyperstack deploy list`
2. Check WebSocket URL: `hyperstack deploy info <build-id>`
3. Ensure no firewall blocking WSS connections

---

## Quick Start Commands Summary

```bash
# One-time setup
cd /Users/adrian/code/defi/hypertek
mkdir hyperstack-test && cd hyperstack-test
cargo init --name hyperstack-test
cargo install --path ../hyperstack-oss/main/cli

# Configure (edit Cargo.toml and hyperstack.toml as shown above)

# Deploy
hyperstack auth login
hyperstack up my-stream

# Generate SDK
hyperstack sdk create typescript my-stream -o ./frontend/src/generated/my-sdk.ts

# Run Rust client
cargo run

# Run TypeScript frontend
cd frontend && npm run dev
```

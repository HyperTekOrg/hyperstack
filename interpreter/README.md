# hyperstack-interpreter

[![crates.io](https://img.shields.io/crates/v/hyperstack-interpreter.svg)](https://crates.io/crates/hyperstack-interpreter)
[![docs.rs](https://docs.rs/hyperstack-interpreter/badge.svg)](https://docs.rs/hyperstack-interpreter)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

AST transformation runtime and VM for HyperStack streaming pipelines.

## Overview

This crate provides the core components for processing Solana blockchain events into typed state projections:

- **AST** - Type-safe definition of state schemas and event handlers
- **Compiler** - Compiles AST specs into optimized bytecode
- **VM** - Executes bytecode to process events and maintain state
- **TypeScript Generation** - Generate client SDKs automatically

## Installation

```toml
[dependencies]
hyperstack-interpreter = "0.1"
```

## Usage

### Define State Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyState {
    pub id: StateId,
    pub metrics: Metrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateId {
    pub key: u64,
}
```

### Compile and Generate TypeScript

```rust
use hyperstack_interpreter::{TypeScriptCompiler, TypeScriptConfig};

let config = TypeScriptConfig::default();
let compiler = TypeScriptCompiler::new(config);
let typescript = compiler.compile(&spec)?;
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `otel` | No | OpenTelemetry integration for distributed tracing and metrics |

## Benefits

- **Type Safety** - Compile-time checking of state structure
- **No String Typos** - Field paths validated at compile time
- **IDE Support** - Full autocomplete and navigation
- **Refactorable** - Rename fields, accessors update automatically

## License

Apache-2.0

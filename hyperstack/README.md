# hyperstack

[![crates.io](https://img.shields.io/crates/v/hyperstack.svg)](https://crates.io/crates/hyperstack)
[![docs.rs](https://docs.rs/hyperstack/badge.svg)](https://docs.rs/hyperstack)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Real-time streaming data pipelines for Solana - transform on-chain events into typed state projections.

## Installation

```toml
[dependencies]
hyperstack = "0.1"
```

Or with all features:

```toml
[dependencies]
hyperstack = { version = "0.1", features = ["full"] }
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `interpreter` | ✅ | AST transformation runtime and VM |
| `spec-macros` | ✅ | Proc-macros for defining stream specifications |
| `server` | ✅ | WebSocket server and projection handlers |
| `sdk` | ❌ | Rust client for connecting to HyperStack servers |
| `full` | ❌ | Enables all features |

## Sub-crates

This is an umbrella crate that re-exports:

- [`hyperstack-interpreter`](https://crates.io/crates/hyperstack-interpreter) - AST transformation runtime
- [`hyperstack-spec-macros`](https://crates.io/crates/hyperstack-spec-macros) - Stream specification macros
- [`hyperstack-server`](https://crates.io/crates/hyperstack-server) - WebSocket server
- [`hyperstack-sdk`](https://crates.io/crates/hyperstack-sdk) - Rust client SDK

## Usage

```rust
use hyperstack::prelude::*;

// Define a stream specification
stream_spec! {
    name: "TokenTracker",
    // ... your specification
}
```

## License

Apache-2.0

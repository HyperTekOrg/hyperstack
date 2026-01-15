# hyperstack

[![crates.io](https://img.shields.io/crates/v/hyperstack.svg)](https://crates.io/crates/hyperstack)
[![docs.rs](https://docs.rs/hyperstack/badge.svg)](https://docs.rs/hyperstack)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Real-time streaming data pipelines for Solana - transform on-chain events into typed state projections.

## Installation

```toml
[dependencies]
hyperstack = "0.2"
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
| `macros` | ✅ | Proc-macros for defining streams |
| `server` | ✅ | WebSocket server and projection handlers |
| `sdk` | ❌ | Rust client for connecting to HyperStack servers |
| `full` | ❌ | Enables all features |

## Sub-crates

This is an umbrella crate that re-exports:

- [`hyperstack-interpreter`](https://crates.io/crates/hyperstack-interpreter) - AST transformation runtime
- [`hyperstack-macros`](https://crates.io/crates/hyperstack-macros) - Stream definition macros
- [`hyperstack-server`](https://crates.io/crates/hyperstack-server) - WebSocket server
- [`hyperstack-sdk`](https://crates.io/crates/hyperstack-sdk) - Rust client SDK

## Usage

```rust
use hyperstack_macros::hyperstack;

// Define a stream
#[hyperstack(idl = "idl.json")]
pub mod my_stream {
    #[entity(name = "MyEntity")]
    #[derive(Stream)]
    struct MyEntity {
        #[map(from = Account::field, primary_key)]
        pub id: String,
    }
}
```

## License

Apache-2.0

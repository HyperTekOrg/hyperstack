# hyperstack-sdk

[![crates.io](https://img.shields.io/crates/v/hyperstack-sdk.svg)](https://crates.io/crates/hyperstack-sdk)
[![docs.rs](https://docs.rs/hyperstack-sdk/badge.svg)](https://docs.rs/hyperstack-sdk)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Rust client SDK for connecting to HyperStack streaming servers.

## Installation

```toml
[dependencies]
hyperstack-sdk = "0.1"
```

## Usage

```rust
use hyperstack_sdk::{HyperStackClient, Subscription};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to a HyperStack server
    let client = HyperStackClient::connect("ws://localhost:8877").await?;
    
    // Subscribe to entity updates by key
    let subscription = client
        .subscribe("MyEntity/kv", Some(json!({"key": "abc"})))
        .await?;
    
    // Process incoming frames
    while let Some(frame) = subscription.next().await {
        println!("Received update: {:?}", frame);
    }
    
    Ok(())
}
```

## Features

- WebSocket-based real-time streaming
- Automatic reconnection handling
- Type-safe frame parsing
- Multiple subscription modes

## Streaming Modes

| Mode | Description |
|------|-------------|
| State | Single shared state object |
| KV | Key-value lookups by entity key |
| List | All entities matching filters |
| Append | Append-only event log |

## Examples

See the `examples/` directory for complete examples:

- **flip/** - Flip game state tracking
- **pump/** - Token launch and trade monitoring

Run examples with:

```bash
cargo run --example flip
cargo run --example pump_new
cargo run --example pump_trades
```

## License

Apache-2.0

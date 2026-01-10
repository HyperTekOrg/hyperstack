# hyperstack-sdk

[![crates.io](https://img.shields.io/crates/v/hyperstack-sdk.svg)](https://crates.io/crates/hyperstack-sdk)
[![docs.rs](https://docs.rs/hyperstack-sdk/badge.svg)](https://docs.rs/hyperstack-sdk)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Rust client SDK for connecting to HyperStack streaming servers.

## Installation

```toml
[dependencies]
hyperstack-sdk = "0.1"
```

### TLS Options

By default, the SDK uses `rustls` for TLS. You can switch to native TLS:

```toml
[dependencies]
hyperstack-sdk = { version = "0.1", default-features = false, features = ["native-tls"] }
```

## Quick Start

```rust
use hyperstack_sdk::{HyperStack, Entity, Update};
use futures_util::StreamExt;

// Import from your generated SDK crate
use my_stack::{PumpfunToken, PumpfunTokenEntity};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to a HyperStack server
    let hs = HyperStack::connect("wss://mainnet.hyperstack.xyz").await?;
    
    // Get a single entity by key
    if let Some(token) = hs.get::<PumpfunTokenEntity>("mint_address").await {
        println!("Token: {:?}", token);
    }
    
    // List all entities
    let tokens = hs.list::<PumpfunTokenEntity>().await;
    println!("Found {} tokens", tokens.len());
    
    // Watch for real-time updates
    let mut stream = hs.watch::<PumpfunTokenEntity>().await;
    while let Some(update) = stream.next().await {
        match update {
            Update::Upsert { key, data } => println!("Updated {}: {:?}", key, data),
            Update::Patch { key, data } => println!("Patched {}: {:?}", key, data),
            Update::Delete { key } => println!("Deleted: {}", key),
        }
    }
    
    Ok(())
}
```

## API Reference

### HyperStack Client

The main client for connecting to HyperStack servers.

```rust
// Simple connection
let hs = HyperStack::connect("wss://example.com").await?;

// With configuration
let hs = HyperStack::builder()
    .url("wss://example.com")
    .auto_reconnect(true)
    .max_reconnect_attempts(10)
    .ping_interval(Duration::from_secs(30))
    .connect()
    .await?;
```

### Core Methods

| Method | Description |
|--------|-------------|
| `get::<E>(key)` | Get a single entity by key |
| `list::<E>()` | Get all entities of type E |
| `watch::<E>()` | Stream all updates for entity type E |
| `watch_key::<E>(key)` | Stream updates for a specific key |
| `connection_state()` | Get current connection state |
| `disconnect()` | Close the connection |

### Entity Trait

The `Entity` trait is implemented by generated SDK code for type-safe access:

```rust
pub trait Entity: Sized + Send + Sync + 'static {
    type Data: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;
    
    const NAME: &'static str;
    
    fn state_view() -> &'static str;
    fn list_view() -> &'static str;
    fn kv_view() -> &'static str;
}
```

### Update Types

When streaming, you receive typed `Update<T>` variants:

```rust
pub enum Update<T> {
    Upsert { key: String, data: T },  // Full entity update
    Patch { key: String, data: T },   // Partial update (merged)
    Delete { key: String },            // Entity removed
}
```

Helper methods:

```rust
update.key()       // Get the entity key
update.data()      // Get data (Some for Upsert/Patch, None for Delete)
update.is_delete() // Check if this is a deletion
```

## Generating a Rust SDK

Use the HyperStack CLI to generate a typed Rust SDK from your spec:

```bash
# Generate SDK crate
hs sdk create rust settlement-game

# With custom output directory
hs sdk create rust settlement-game --output ./crates/game-sdk

# With custom crate name
hs sdk create rust settlement-game --crate-name game-sdk
```

This generates a crate with:

```
generated/settlement-game-stack/
├── Cargo.toml
└── src/
    ├── lib.rs      # Re-exports
    ├── types.rs    # Data structs (SettlementGame, Player, etc.)
    └── entity.rs   # Entity trait implementations
```

Add the generated crate to your `Cargo.toml`:

```toml
[dependencies]
hyperstack-sdk = "0.1"
settlement-game-stack = { path = "./generated/settlement-game-stack" }
```

Then use it:

```rust
use hyperstack_sdk::HyperStack;
use settlement_game_stack::{SettlementGame, SettlementGameEntity};

let hs = HyperStack::connect("wss://example.com").await?;
let game = hs.get::<SettlementGameEntity>("game_id").await;
```

## Connection Management

### Auto-Reconnection

The SDK automatically reconnects on connection loss with exponential backoff:

```rust
let hs = HyperStack::builder()
    .url("wss://example.com")
    .auto_reconnect(true)
    .reconnect_intervals(vec![
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(5),
        Duration::from_secs(10),
    ])
    .max_reconnect_attempts(20)
    .connect()
    .await?;
```

### Connection State

Monitor connection health:

```rust
let state = hs.connection_state().await;
match state {
    ConnectionState::Connected => println!("Connected"),
    ConnectionState::Connecting => println!("Connecting..."),
    ConnectionState::Reconnecting { attempt } => println!("Reconnecting (attempt {})", attempt),
    ConnectionState::Disconnected => println!("Disconnected"),
    ConnectionState::Failed { error } => println!("Failed: {}", error),
}
```

## Streaming Modes

| Mode | View | Description |
|------|------|-------------|
| State | `Entity/state` | Single shared state object |
| KV | `Entity/kv` | Key-value lookups by entity key |
| List | `Entity/list` | All entities matching filters |
| Append | `Entity/append` | Append-only event log |

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

MIT

# hyperstack-stacks

[![crates.io](https://img.shields.io/crates/v/hyperstack-stacks.svg)](https://crates.io/crates/hyperstack-stacks)
[![docs.rs](https://docs.rs/hyperstack-stacks/badge.svg)](https://docs.rs/hyperstack-stacks)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Protocol stacks for Hyperstack - ready-to-use Solana data streams.

## Installation

```toml
[dependencies]
hyperstack-stacks = "0.2"
```

Or with specific features:

```toml
[dependencies]
hyperstack-stacks = { version = "0.2", default-features = false, features = ["pumpfun"] }
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `pumpfun` | Yes | PumpFun token streaming |
| `full` | No | Enables all stacks |

## Usage

```rust
use hyperstack_sdk::prelude::*;
use hyperstack_stacks::pumpfun::{PumpfunToken, PumpfunTokenEntity};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hs = HyperStack::connect("wss://mainnet.hyperstack.xyz").await?;
    
    // List all tokens
    let tokens = hs.list::<PumpfunTokenEntity>().await;
    println!("Found {} tokens", tokens.len());
    
    // Watch for real-time updates
    let mut stream = hs.watch::<PumpfunTokenEntity>();
    while let Some(update) = stream.next().await {
        match update {
            Update::Upsert { key, data } => {
                println!("Token {}: {:?}", key, data.info.name);
            }
            Update::Delete { key } => {
                println!("Token {} removed", key);
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Available Stacks

### PumpFun Token Stack

Real-time streaming data for PumpFun tokens on Solana.

```rust
use hyperstack_stacks::pumpfun::{PumpfunToken, PumpfunTokenEntity};
```

**Entity: `PumpfunToken`**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `PumpfunTokenId` | Token identifiers (mint, bonding curve) |
| `info` | `PumpfunTokenInfo` | Token metadata (name, symbol, URI) |
| `reserves` | `PumpfunTokenReserves` | Current reserve state and pricing |
| `trading` | `PumpfunTokenTrading` | Trading statistics and metrics |
| `events` | `PumpfunTokenEvents` | Recent buy/sell/create events |

## Dependencies

This crate depends on [`hyperstack-sdk`](https://crates.io/crates/hyperstack-sdk) for the core streaming functionality.

## License

MIT

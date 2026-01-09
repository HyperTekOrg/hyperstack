# hyperstack-macros

[![crates.io](https://img.shields.io/crates/v/hyperstack-macros.svg)](https://crates.io/crates/hyperstack-macros)
[![docs.rs](https://docs.rs/hyperstack-macros/badge.svg)](https://docs.rs/hyperstack-macros)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Procedural macros for defining HyperStack streams.

## Overview

This crate provides the `#[hyperstack]` attribute macro that transforms annotated Rust structs into full streaming pipeline specifications, including:

- State struct generation with field accessors
- Handler creation functions for event processing
- IDL/Proto parser integration for Solana programs
- Automatic AST serialization for deployment

## Installation

```toml
[dependencies]
hyperstack-macros = "0.1"
```

## Usage

### IDL-based Stream

```rust
use hyperstack_macros::{hyperstack, Stream};

#[hyperstack(idl = "idl.json")]
pub mod my_stream {
    #[entity(name = "MyEntity")]
    #[derive(Stream)]
    struct Entity {
        #[map(from = "MyAccount", field = "value")]
        pub value: u64,
        
        #[map(from = "MyAccount", field = "owner")]
        pub owner: String,
    }
}
```

### Proto-based Stream

```rust
#[hyperstack(proto = ["events.proto"])]
pub mod my_stream {
    // entity structs
}
```

## Supported Attributes

| Attribute | Description |
|-----------|-------------|
| `#[map(...)]` | Map from account fields |
| `#[from_instruction(...)]` | Map from instruction fields |
| `#[event(...)]` | Capture instruction events |
| `#[snapshot(...)]` | Capture entire source data |
| `#[aggregate(...)]` | Aggregate field values |
| `#[computed(...)]` | Computed fields from other fields |
| `#[derive_from(...)]` | Derive values from instructions |

## Generated Output

The macro generates:

- `{EntityName}State` struct with all fields
- `fields::` module with field accessors
- `create_spec()` function returning `TypedStreamSpec`
- Handler creation functions for each source

## License

Apache-2.0

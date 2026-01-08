# hyperstack-spec-macros

[![crates.io](https://img.shields.io/crates/v/hyperstack-spec-macros.svg)](https://crates.io/crates/hyperstack-spec-macros)
[![docs.rs](https://docs.rs/hyperstack-spec-macros/badge.svg)](https://docs.rs/hyperstack-spec-macros)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Procedural macros for defining HyperStack stream specifications.

## Overview

This crate provides the `#[stream_spec]` attribute macro that transforms annotated Rust structs into full streaming pipeline specifications, including:

- State struct generation with field accessors
- Handler creation functions for event processing
- IDL/Proto parser integration for Solana programs
- Automatic AST serialization for deployment

## Installation

```toml
[dependencies]
hyperstack-spec-macros = "0.1"
```

## Usage

### IDL-based Pipeline

```rust
use hyperstack_spec_macros::{stream_spec, StreamSection};

#[stream_spec(idl = "idl.json")]
pub mod my_pipeline {
    #[entity(name = "MyEntity")]
    #[derive(StreamSection)]
    struct Entity {
        #[map(from = "MyAccount", field = "value")]
        pub value: u64,
        
        #[map(from = "MyAccount", field = "owner")]
        pub owner: String,
    }
}
```

### Proto-based Pipeline

```rust
#[stream_spec(proto = ["events.proto"])]
pub mod my_pipeline {
    // entity structs
}
```

## Supported Attributes

| Attribute | Description |
|-----------|-------------|
| `#[map(...)]` | Map from account fields |
| `#[map_instruction(...)]` | Map from instruction fields |
| `#[event(...)]` | Capture instruction events |
| `#[capture(...)]` | Capture entire source data |
| `#[aggregate(...)]` | Aggregate field values |
| `#[computed(...)]` | Computed fields from other fields |
| `#[track_from(...)]` | Track values from instructions |

## Generated Output

The macro generates:

- `{EntityName}State` struct with all fields
- `fields::` module with field accessors
- `create_spec()` function returning `TypedStreamSpec`
- Handler creation functions for each source

## License

Apache-2.0

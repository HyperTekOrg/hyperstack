# Extended Resolve Macro - URL Resolution Support

## Overview

This PR extends the `#[resolve]` macro to support URL-based data fetching in addition to the existing Token metadata resolution via DAS API.

## New Capability

The `#[resolve]` macro now supports fetching and extracting data from HTTP URLs:

```rust
// Token resolver (existing functionality)
#[resolve(address = "oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp")]
pub ore_metadata: Option<TokenMetadata>,

// URL resolver (new)
#[resolve(url = info.uri, extract = "image")]
pub resolved_image: Option<String>,

// URL resolver with HTTP method
#[resolve(url = api.endpoint, method = POST, extract = "data.result")]
pub result: Option<String>,
```

## Resolver Type Detection

The macro automatically determines the resolver type based on parameters:

| Parameters | Resolver Type |
|------------|---------------|
| `url = ...` | URL Resolver (HTTP fetch + JSON extraction) |
| `address = ...` or `from = ...` | Token Resolver (DAS API) |

Parameters are mutually exclusive - specifying both `url` and `address`/`from` is a compile error.

## URL Resolver Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `url` | Yes | Field path containing the URL to fetch (e.g., `info.uri`) |
| `extract` | Yes | JSON path to extract from response (e.g., `"image"`, `"data.nested.field"`) |
| `method` | No | HTTP method: `GET` (default) or `POST` |

## Example Usage

```rust
pub struct TokenInfo {
    #[from_instruction([Create::uri], strategy = SetOnce)]
    pub uri: Option<String>,

    // Fetch metadata JSON from uri and extract the "image" field
    #[resolve(url = info.uri, extract = "image")]
    pub resolved_image: Option<String>,
}
```

## Changes

### Files Modified

- **`hyperstack-macros/src/parse/attributes.rs`**
  - Added `url` and `method` fields to `ResolveAttributeArgs`
  - Updated parser to handle dot-path syntax for URL field references
  - Added validation for mutually exclusive parameters

- **`hyperstack-macros/src/stream_spec/entity.rs`**
  - Updated resolve branch to create `ResolverType::Url` when `url` parameter is present

- **`hyperstack-macros/src/stream_spec/sections.rs`**
  - Same updates as entity.rs with section-name prefixing for unqualified paths

- **`stacks/pumpfun/src/stack.rs`**
  - Added example usage of URL resolution
# Changelog

## [0.3.7](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.6...hyperstack-macros-v0.3.7) (2026-01-26)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.3.6](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.5...hyperstack-macros-v0.3.6) (2026-01-26)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.3.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.4...hyperstack-macros-v0.3.5) (2026-01-24)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.3.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.3...hyperstack-macros-v0.3.4) (2026-01-24)


### Bug Fixes

* **macros:** add explicit type annotation to account_names vector ([76a124e](https://github.com/HyperTekOrg/hyperstack/commit/76a124ef623c6aabd4d88ae688701403105a80dd))

## [0.3.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.2...hyperstack-macros-v0.3.3) (2026-01-23)


### Features

* add derived view support to React SDK and macros ([5f6414f](https://github.com/HyperTekOrg/hyperstack/commit/5f6414f879f2891be2d8ee5c16173cf83ddf2ea9))
* add view pipeline for derived views like `latest`, `top10` ([f2f72fa](https://github.com/HyperTekOrg/hyperstack/commit/f2f72fa8894d2a38a13e8ee05791b7e4124977ea))
* **macros:** add #[view] attribute for declarative view definitions on entities ([3f0bdc5](https://github.com/HyperTekOrg/hyperstack/commit/3f0bdc51d7945c32082ffa8997362328c7b26022))
* **macros:** support tuple structs, 8-byte discriminators, and optional error messages in IDL ([090b5d6](https://github.com/HyperTekOrg/hyperstack/commit/090b5d62999e7bbae2dfb577a0d028b6675def01))
* **sdk:** add sorted view support with server-side subscribed frame ([1a7d83f](https://github.com/HyperTekOrg/hyperstack/commit/1a7d83fe4000c26d282f2df9ce95f9d79414014d))
* **server:** add slot-based sequence ordering for list views ([892c3d5](https://github.com/HyperTekOrg/hyperstack/commit/892c3d526c71df4c4d848142908ce511e302e082))


### Bug Fixes

* Account lookup ([bdf8b55](https://github.com/HyperTekOrg/hyperstack/commit/bdf8b5564619695575503e817507c0c8238cecac))
* convert field names to camelCase for JSON serialization ([3dc05a4](https://github.com/HyperTekOrg/hyperstack/commit/3dc05a4eddebca9636827562f9793fdf1c16c1c9))
* Logging ([b727602](https://github.com/HyperTekOrg/hyperstack/commit/b727602bd0a232fcede8bfdeea4c9ec3d060483d))
* **macros:** convert lookup_indexes field_name to camelCase ([0c65c07](https://github.com/HyperTekOrg/hyperstack/commit/0c65c071adb36a768708eb53d05f3dcf0fb3c3b6))
* **macros:** simplify account mapping and warn on IDL mismatch ([597ea1e](https://github.com/HyperTekOrg/hyperstack/commit/597ea1e155e9fa19572f1d32a4c3089a7c7c57ca))

## [0.3.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.1...hyperstack-macros-v0.3.2) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.3.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.3.0...hyperstack-macros-v0.3.1) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.3.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.5...hyperstack-macros-v0.3.0) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.2.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.4...hyperstack-macros-v0.2.5) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.2.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.3...hyperstack-macros-v0.2.4) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.2.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.2...hyperstack-macros-v0.2.3) (2026-01-18)


### Features

* add append hints to frame protocol for granular array updates ([ce2213f](https://github.com/HyperTekOrg/hyperstack/commit/ce2213fc5a2c242cb4833ab417ff3d71f918812f))
* **server:** add trace context propagation and expanded telemetry ([0dbd8ed](https://github.com/HyperTekOrg/hyperstack/commit/0dbd8ed49780dd2f8f793b6af2425b47d9ccb151))


### Bug Fixes

* preserve VM state across reconnections and add memory management ([7fba770](https://github.com/HyperTekOrg/hyperstack/commit/7fba770df913dd0fbd06e43b402c6c288b25acbb))

## [0.2.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.1...hyperstack-macros-v0.2.2) (2026-01-16)


### Features

* **interpreter:** add memory limits and LRU eviction to prevent unbounded growth ([33198a6](https://github.com/HyperTekOrg/hyperstack/commit/33198a69833de6e57f0c5fe568b0714a2105e987))
* **interpreter:** add staleness detection to reject out-of-order gRPC updates ([d693f42](https://github.com/HyperTekOrg/hyperstack/commit/d693f421742258bbbd3528ffbbd4731d638c992b))


### Bug Fixes

* Clippy errors ([d6a9f4d](https://github.com/HyperTekOrg/hyperstack/commit/d6a9f4d27f619d05189f421e214f6eacb8c19542))

## [0.2.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.2.0...hyperstack-macros-v0.2.1) (2026-01-16)


### Features

* add gRPC stream reconnection with exponential backoff ([48e3ec7](https://github.com/HyperTekOrg/hyperstack/commit/48e3ec7a952399135a84323da78cdc499804bce9))

## [0.2.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.11...hyperstack-macros-v0.2.0) (2026-01-15)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.1.11](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.10...hyperstack-macros-v0.1.11) (2026-01-14)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.1.10](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.9...hyperstack-macros-v0.1.10) (2026-01-13)


### Features

* Add deterministic sorting ([e775f59](https://github.com/HyperTekOrg/hyperstack/commit/e775f598a95165b2dd5504be67d24e7b1dabc766))

## [0.1.9](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.8...hyperstack-macros-v0.1.9) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.1.8](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.6...hyperstack-macros-v0.1.8) (2026-01-13)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Map syntax ([8a5eaad](https://github.com/HyperTekOrg/hyperstack/commit/8a5eaadf5642dc7e569b2591e8e051c728a6eb9f))
* Module name snake case ([72348a4](https://github.com/HyperTekOrg/hyperstack/commit/72348a42ee3988e94873db0d24317f3e661e093d))
* Preserve integer types in computed field expressions ([616f042](https://github.com/HyperTekOrg/hyperstack/commit/616f04288637a84a4eed0febebf9867e06d134cb))
* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))

## [0.1.6](https://github.com/HyperTekOrg/hyperstack/compare/v0.1.5...v0.1.6) (2026-01-13)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Map syntax ([8a5eaad](https://github.com/HyperTekOrg/hyperstack/commit/8a5eaadf5642dc7e569b2591e8e051c728a6eb9f))
* Module name snake case ([72348a4](https://github.com/HyperTekOrg/hyperstack/commit/72348a42ee3988e94873db0d24317f3e661e093d))
* Preserve integer types in computed field expressions ([616f042](https://github.com/HyperTekOrg/hyperstack/commit/616f04288637a84a4eed0febebf9867e06d134cb))
* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))

## [0.1.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.4...hyperstack-macros-v0.1.5) (2026-01-09)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Map syntax ([8a5eaad](https://github.com/HyperTekOrg/hyperstack/commit/8a5eaadf5642dc7e569b2591e8e051c728a6eb9f))
* Module name snake case ([72348a4](https://github.com/HyperTekOrg/hyperstack/commit/72348a42ee3988e94873db0d24317f3e661e093d))

## [0.1.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.2...hyperstack-macros-v0.1.4) (2026-01-09)


### Miscellaneous Chores

* **hyperstack-macros:** Synchronize hyperstack versions

## [0.1.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-macros-v0.1.1...hyperstack-macros-v0.1.2) (2026-01-09)


### Bug Fixes

* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))

## [0.1.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-spec-macros-v0.1.0...hyperstack-spec-macros-v0.1.1) (2026-01-09)


### Features

* Better sdk types during generation ([f9555ef](https://github.com/HyperTekOrg/hyperstack/commit/f9555ef440eb9271a147d178d8b3554cf532b9c7))


### Bug Fixes

* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))

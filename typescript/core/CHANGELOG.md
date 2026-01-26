# Changelog

## [0.3.7](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.6...hyperstack-typescript-v0.3.7) (2026-01-26)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.3.6](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.5...hyperstack-typescript-v0.3.6) (2026-01-26)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.3.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.4...hyperstack-typescript-v0.3.5) (2026-01-24)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.3.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.3...hyperstack-typescript-v0.3.4) (2026-01-24)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.3.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.2...hyperstack-typescript-v0.3.3) (2026-01-23)


### Features

* **react:** add configurable frame buffering to reduce render churn ([c4bdb13](https://github.com/HyperTekOrg/hyperstack/commit/c4bdb13bf8efa085b8105c1fbbdc1e19127e6590))
* **sdk:** add sorted view support with server-side subscribed frame ([1a7d83f](https://github.com/HyperTekOrg/hyperstack/commit/1a7d83fe4000c26d282f2df9ce95f9d79414014d))

## [0.3.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.1...hyperstack-typescript-v0.3.2) (2026-01-20)


### Bug Fixes

* prevent duplicate WebSocket subscriptions from same client ([8135fdf](https://github.com/HyperTekOrg/hyperstack/commit/8135fdf28461b1906a03fe78c3f9ae50362ccb96))
* send snapshots in batches for faster initial page loads ([d4a8c40](https://github.com/HyperTekOrg/hyperstack/commit/d4a8c405bbd5859f40825d99a3b044c64ede6985))
* update SDKs to detect and decompress raw gzip binary frames ([2441b54](https://github.com/HyperTekOrg/hyperstack/commit/2441b54e7f3dbf53cea428e0aa6bcd81b9a06e60))

## [0.3.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.3.0...hyperstack-typescript-v0.3.1) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.3.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.5...hyperstack-typescript-v0.3.0) (2026-01-20)


### ⚠ BREAKING CHANGES

* EntityStore removed from core exports, replaced by StorageAdapter interface

### Features

* add gzip compression for large WebSocket payloads ([cb694e9](https://github.com/HyperTekOrg/hyperstack/commit/cb694e9ef74ff99345e5f054820207f743d55e1d))
* Pluggable storage adapter architecture for React SDK ([60dac5e](https://github.com/HyperTekOrg/hyperstack/commit/60dac5e2d22f2dc388fc229efdf4068a95aa756f))

## [0.2.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.4...hyperstack-typescript-v0.2.5) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.2.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.3...hyperstack-typescript-v0.2.4) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.2.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.2...hyperstack-typescript-v0.2.3) (2026-01-18)


### Features

* add append hints to frame protocol for granular array updates ([ce2213f](https://github.com/HyperTekOrg/hyperstack/commit/ce2213fc5a2c242cb4833ab417ff3d71f918812f))
* implement proper unsubscribe support across server and all SDKs ([81118cb](https://github.com/HyperTekOrg/hyperstack/commit/81118cb103720bdf8424cb71aab63d24d26e434c))
* **sdk:** add configurable store size limits with LRU eviction ([3e91148](https://github.com/HyperTekOrg/hyperstack/commit/3e91148b68c02b97da60dc9d12f1a45369895e7d))
* **sdk:** add snapshot frame support for batched initial data ([bf7cafe](https://github.com/HyperTekOrg/hyperstack/commit/bf7cafe9bcd0b8f255cd710b622d412476acb6a9))

## [0.2.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.1...hyperstack-typescript-v0.2.2) (2026-01-16)


### Features

* **interpreter:** add memory limits and LRU eviction to prevent unbounded growth ([33198a6](https://github.com/HyperTekOrg/hyperstack/commit/33198a69833de6e57f0c5fe568b0714a2105e987))

## [0.2.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.2.0...hyperstack-typescript-v0.2.1) (2026-01-16)


### Miscellaneous Chores

* **hyperstack-typescript:** Synchronize hyperstack versions

## [0.2.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-typescript-v0.1.11...hyperstack-typescript-v0.2.0) (2026-01-15)


### Bug Fixes

* **ci:** add basic tests for core SDK and skip react until core is published ([89def14](https://github.com/HyperTekOrg/hyperstack/commit/89def14ec05fe9265059ee58a8d9b169f32e03ec))

# Changelog

## [0.2.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.2.3...hyperstack-sdk-v0.2.4) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.2.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.2.2...hyperstack-sdk-v0.2.3) (2026-01-18)


### Features

* add append hints to frame protocol for granular array updates ([ce2213f](https://github.com/HyperTekOrg/hyperstack/commit/ce2213fc5a2c242cb4833ab417ff3d71f918812f))
* implement proper unsubscribe support across server and all SDKs ([81118cb](https://github.com/HyperTekOrg/hyperstack/commit/81118cb103720bdf8424cb71aab63d24d26e434c))
* **sdk:** add configurable store size limits with LRU eviction ([3e91148](https://github.com/HyperTekOrg/hyperstack/commit/3e91148b68c02b97da60dc9d12f1a45369895e7d))
* **sdk:** add snapshot frame support for batched initial data ([bf7cafe](https://github.com/HyperTekOrg/hyperstack/commit/bf7cafe9bcd0b8f255cd710b622d412476acb6a9))

## [0.2.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.2.1...hyperstack-sdk-v0.2.2) (2026-01-16)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.2.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.2.0...hyperstack-sdk-v0.2.1) (2026-01-16)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.2.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.11...hyperstack-sdk-v0.2.0) (2026-01-15)


### ⚠ BREAKING CHANGES

* Wire protocol simplified - list frames no longer wrap data in {id, order, item}. Data is now sent directly.

### Features

* **rust-sdk:** add deep merge for patches and expose raw patch data ([000f2a2](https://github.com/HyperTekOrg/hyperstack/commit/000f2a2092872aca689e0c00b17f069704f506f6))
* **rust-sdk:** add lazy streams with chainable filter/map operators ([e7fd71a](https://github.com/HyperTekOrg/hyperstack/commit/e7fd71a1a430a6e28db77fa79942c27aba29bf28))
* **rust-sdk:** add prelude module, rich updates, batch key watching, and DX improvements ([ac8d513](https://github.com/HyperTekOrg/hyperstack/commit/ac8d513b234544c165a24def685533c9cc31f8be))


### Bug Fixes

* remove deprecated kv mode from SDKs and documentation ([2097af0](https://github.com/HyperTekOrg/hyperstack/commit/2097af05165eed4a7d9b6ef4ede1b5722ab90215))


### Code Refactoring

* remove Mode::Kv and simplify websocket frame structure ([f1a2b81](https://github.com/HyperTekOrg/hyperstack/commit/f1a2b81f24eeda9a81b5fc0738ef78a5741b687b))

## [0.1.11](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.10...hyperstack-sdk-v0.1.11) (2026-01-14)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.1.10](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.9...hyperstack-sdk-v0.1.10) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.1.9](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.8...hyperstack-sdk-v0.1.9) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-sdk:** Synchronize hyperstack versions

## [0.1.8](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.6...hyperstack-sdk-v0.1.8) (2026-01-13)


### Features

* Refactor API with Entity trait and connection management ([d5c010c](https://github.com/HyperTekOrg/hyperstack/commit/d5c010ca96e5a125943f742af424e2693e2afe47))


### Bug Fixes

* Add hyperstack-sdk to CI ([e5f3f4c](https://github.com/HyperTekOrg/hyperstack/commit/e5f3f4c7ac144000683297ec79efd946bf626b07))
* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Extract entity name from view path and unwrap item data ([289ce66](https://github.com/HyperTekOrg/hyperstack/commit/289ce667c1f28bd67c8179e7113c130a347d7f2a))
* Use BroadcastStream for proper async polling ([9dbbc42](https://github.com/HyperTekOrg/hyperstack/commit/9dbbc4251d788d889adcbdb31f4e22987b48e05b))

## [0.1.6](https://github.com/HyperTekOrg/hyperstack/compare/v0.1.5...v0.1.6) (2026-01-13)


### Features

* Refactor API with Entity trait and connection management ([d5c010c](https://github.com/HyperTekOrg/hyperstack/commit/d5c010ca96e5a125943f742af424e2693e2afe47))


### Bug Fixes

* Add hyperstack-sdk to CI ([e5f3f4c](https://github.com/HyperTekOrg/hyperstack/commit/e5f3f4c7ac144000683297ec79efd946bf626b07))
* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Extract entity name from view path and unwrap item data ([289ce66](https://github.com/HyperTekOrg/hyperstack/commit/289ce667c1f28bd67c8179e7113c130a347d7f2a))
* Use BroadcastStream for proper async polling ([9dbbc42](https://github.com/HyperTekOrg/hyperstack/commit/9dbbc4251d788d889adcbdb31f4e22987b48e05b))

## [0.1.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.4...hyperstack-sdk-v0.1.5) (2026-01-09)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))

## [0.1.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-sdk-v0.1.2...hyperstack-sdk-v0.1.4) (2026-01-09)


### Bug Fixes

* Add hyperstack-sdk to CI ([e5f3f4c](https://github.com/HyperTekOrg/hyperstack/commit/e5f3f4c7ac144000683297ec79efd946bf626b07))

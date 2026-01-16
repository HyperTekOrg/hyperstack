# Changelog

## [0.2.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.2.1...hyperstack-server-v0.2.2) (2026-01-16)


### Miscellaneous Chores

* **hyperstack-server:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.2.1 to 0.2.2

## [0.2.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.2.0...hyperstack-server-v0.2.1) (2026-01-16)


### Features

* add gRPC stream reconnection with exponential backoff ([48e3ec7](https://github.com/HyperTekOrg/hyperstack/commit/48e3ec7a952399135a84323da78cdc499804bce9))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.2.0 to 0.2.1

## [0.2.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.11...hyperstack-server-v0.2.0) (2026-01-15)


### ⚠ BREAKING CHANGES

* Wire protocol simplified - list frames no longer wrap data in {id, order, item}. Data is now sent directly.

### Features

* **server:** add entity cache for snapshot-on-subscribe ([5342720](https://github.com/HyperTekOrg/hyperstack/commit/53427201fb1dbd69f918b07dfc5355c89c2a7694))


### Bug Fixes

* remove deprecated kv mode from SDKs and documentation ([2097af0](https://github.com/HyperTekOrg/hyperstack/commit/2097af05165eed4a7d9b6ef4ede1b5722ab90215))


### Code Refactoring

* remove Mode::Kv and simplify websocket frame structure ([f1a2b81](https://github.com/HyperTekOrg/hyperstack/commit/f1a2b81f24eeda9a81b5fc0738ef78a5741b687b))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.11 to 0.2.0

## [0.1.11](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.10...hyperstack-server-v0.1.11) (2026-01-14)


### Miscellaneous Chores

* **hyperstack-server:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.10 to 0.1.11

## [0.1.10](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.9...hyperstack-server-v0.1.10) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-server:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.9 to 0.1.10

## [0.1.9](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.8...hyperstack-server-v0.1.9) (2026-01-13)


### Bug Fixes

* Otel feature flag on timing ([42626fb](https://github.com/HyperTekOrg/hyperstack/commit/42626fbc9db0e48f5ab966741c5e71bc34718059))
* Variable incorrectly named ([826fb82](https://github.com/HyperTekOrg/hyperstack/commit/826fb825f12dd17f96be439aa99d59d3a60bb96c))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.8 to 0.1.9

## [0.1.8](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.7...hyperstack-server-v0.1.8) (2026-01-13)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))
* Feature flag import ([a5c3160](https://github.com/HyperTekOrg/hyperstack/commit/a5c3160cedff5c678c23f266627d2fa7bebc890c))
* Missing import ([dc9a92d](https://github.com/HyperTekOrg/hyperstack/commit/dc9a92d7f685692746d8c6349da440b2e21028aa))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.6 to 0.1.8

## [0.1.7](https://github.com/HyperTekOrg/hyperstack/compare/v0.1.6...v0.1.7) (2026-01-13)


### Bug Fixes

* Feature flag import ([a5c3160](https://github.com/HyperTekOrg/hyperstack/commit/a5c3160cedff5c678c23f266627d2fa7bebc890c))
* Missing import ([dc9a92d](https://github.com/HyperTekOrg/hyperstack/commit/dc9a92d7f685692746d8c6349da440b2e21028aa))

## [0.1.6](https://github.com/HyperTekOrg/hyperstack/compare/v0.1.5...v0.1.6) (2026-01-13)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.5 to 0.1.6

## [0.1.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.4...hyperstack-server-v0.1.5) (2026-01-09)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.4 to 0.1.5

## [0.1.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.2...hyperstack-server-v0.1.4) (2026-01-09)


### Miscellaneous Chores

* **hyperstack-server:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.2 to 0.1.4

## [0.1.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.1...hyperstack-server-v0.1.2) (2026-01-09)


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.1 to 0.1.2

## [0.1.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-server-v0.1.0...hyperstack-server-v0.1.1) (2026-01-09)


### Bug Fixes

* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-interpreter bumped from 0.1.0 to 0.1.1

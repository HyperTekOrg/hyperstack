# Changelog

## [0.3.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.3.1...hyperstack-interpreter-v0.3.2) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.3.1 to 0.3.2

## [0.3.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.3.0...hyperstack-interpreter-v0.3.1) (2026-01-20)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.3.0 to 0.3.1

## [0.3.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.5...hyperstack-interpreter-v0.3.0) (2026-01-20)


### Features

* **cli:** add --module flag for Rust SDK generation ([42812e6](https://github.com/HyperTekOrg/hyperstack/commit/42812e673d5b763792b96937d8dd6dee20314253))
* **sdk:** add default export to generated TypeScript SDK ([b24f39f](https://github.com/HyperTekOrg/hyperstack/commit/b24f39f0899bfe53d4307f5b0fa06733178006e2))


### Bug Fixes

* remove needless borrow in rust codegen ([398047b](https://github.com/HyperTekOrg/hyperstack/commit/398047b5c8308ccb05c4426ffecbdd1daf6d6f7b))
* remove unused serde_helpers module from Rust SDK generator ([57e2d13](https://github.com/HyperTekOrg/hyperstack/commit/57e2d13dbcd9bbceaa5cd5bbaf8e1d37f7df99a7))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.5 to 0.3.0

## [0.2.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.4...hyperstack-interpreter-v0.2.5) (2026-01-19)


### Bug Fixes

* emit canonical log data as structured field for OTEL/Axiom parsing ([247e807](https://github.com/HyperTekOrg/hyperstack/commit/247e807019b793a2194d3c9d670c4ab2a01615ac))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.4 to 0.2.5

## [0.2.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.3...hyperstack-interpreter-v0.2.4) (2026-01-19)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.3 to 0.2.4

## [0.2.3](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.2...hyperstack-interpreter-v0.2.3) (2026-01-18)


### Features

* add append hints to frame protocol for granular array updates ([ce2213f](https://github.com/HyperTekOrg/hyperstack/commit/ce2213fc5a2c242cb4833ab417ff3d71f918812f))
* **interpreter:** add canonical logging and OpenTelemetry metrics ([e07de40](https://github.com/HyperTekOrg/hyperstack/commit/e07de40b0a4523dea4958b485b493aed8bbc20b6))
* **interpreter:** implement granular dirty tracking for field emissions ([c490c9c](https://github.com/HyperTekOrg/hyperstack/commit/c490c9ccb912f872ab92fadbfab674fc3ba56090))


### Bug Fixes

* increase cache limits 5x to reduce eviction pressure ([49ed3c4](https://github.com/HyperTekOrg/hyperstack/commit/49ed3c4148fbbdc8ad61817ebf31d5989552b181))
* **interpreter:** add bounded LRU caches to prevent unbounded memory growth ([4d9042e](https://github.com/HyperTekOrg/hyperstack/commit/4d9042e2ca115fe41827fcdeac037bea8a1b5589))
* reduce memory allocations in VM and projector ([d265a4f](https://github.com/HyperTekOrg/hyperstack/commit/d265a4fc358799f33d549412932cce9919b5dc56))
* use derive macro for LogLevel Default impl ([c2e30ed](https://github.com/HyperTekOrg/hyperstack/commit/c2e30ed0e1968e00f1b789e7d2cfb04dd4cb4867))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.2 to 0.2.3

## [0.2.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.1...hyperstack-interpreter-v0.2.2) (2026-01-16)


### Features

* **interpreter:** add memory limits and LRU eviction to prevent unbounded growth ([33198a6](https://github.com/HyperTekOrg/hyperstack/commit/33198a69833de6e57f0c5fe568b0714a2105e987))
* **interpreter:** add staleness detection to reject out-of-order gRPC updates ([d693f42](https://github.com/HyperTekOrg/hyperstack/commit/d693f421742258bbbd3528ffbbd4731d638c992b))


### Bug Fixes

* Clippy errors ([d6a9f4d](https://github.com/HyperTekOrg/hyperstack/commit/d6a9f4d27f619d05189f421e214f6eacb8c19542))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.1 to 0.2.2

## [0.2.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.2.0...hyperstack-interpreter-v0.2.1) (2026-01-16)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.2.0 to 0.2.1

## [0.2.0](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.11...hyperstack-interpreter-v0.2.0) (2026-01-15)


### Bug Fixes

* **interpreter:** make all TypeScript interface fields optional for patch semantics ([d2d959c](https://github.com/HyperTekOrg/hyperstack/commit/d2d959c2d02ceff4c2cf0c76d147df770222cf25))
* remove deprecated kv mode from SDKs and documentation ([2097af0](https://github.com/HyperTekOrg/hyperstack/commit/2097af05165eed4a7d9b6ef4ede1b5722ab90215))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.11 to 0.2.0

## [0.1.11](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.10...hyperstack-interpreter-v0.1.11) (2026-01-14)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.10 to 0.1.11

## [0.1.10](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.9...hyperstack-interpreter-v0.1.10) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.9 to 0.1.10

## [0.1.9](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.8...hyperstack-interpreter-v0.1.9) (2026-01-13)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.8 to 0.1.9

## [0.1.8](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.6...hyperstack-interpreter-v0.1.8) (2026-01-13)


### Features

* Add Rust codegen module for SDK generation ([24fac1c](https://github.com/HyperTekOrg/hyperstack/commit/24fac1cc894729ec44596ddadb969fce79dafbd4))
* Better sdk types during generation ([f9555ef](https://github.com/HyperTekOrg/hyperstack/commit/f9555ef440eb9271a147d178d8b3554cf532b9c7))


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))
* Handle root section case-insensitively and flatten fields ([1cf7110](https://github.com/HyperTekOrg/hyperstack/commit/1cf7110a28450a63b607007237ab46a9a6125bf5))
* Naming issues in generated sdk ([179da1f](https://github.com/HyperTekOrg/hyperstack/commit/179da1f2f6c8c75f99c35c0fb90b38576ffc19e2))
* Preserve integer types in computed field expressions ([616f042](https://github.com/HyperTekOrg/hyperstack/commit/616f04288637a84a4eed0febebf9867e06d134cb))
* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))
* Update typescript package name ([6267eae](https://github.com/HyperTekOrg/hyperstack/commit/6267eaeb19e00a3e1c1f76fca417f56170edafb9))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.6 to 0.1.8

## [0.1.6](https://github.com/HyperTekOrg/hyperstack/compare/v0.1.5...v0.1.6) (2026-01-13)


### Features

* Add Rust codegen module for SDK generation ([24fac1c](https://github.com/HyperTekOrg/hyperstack/commit/24fac1cc894729ec44596ddadb969fce79dafbd4))
* Better sdk types during generation ([f9555ef](https://github.com/HyperTekOrg/hyperstack/commit/f9555ef440eb9271a147d178d8b3554cf532b9c7))


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))
* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))
* Handle root section case-insensitively and flatten fields ([1cf7110](https://github.com/HyperTekOrg/hyperstack/commit/1cf7110a28450a63b607007237ab46a9a6125bf5))
* Naming issues in generated sdk ([179da1f](https://github.com/HyperTekOrg/hyperstack/commit/179da1f2f6c8c75f99c35c0fb90b38576ffc19e2))
* Preserve integer types in computed field expressions ([616f042](https://github.com/HyperTekOrg/hyperstack/commit/616f04288637a84a4eed0febebf9867e06d134cb))
* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))
* Update typescript package name ([6267eae](https://github.com/HyperTekOrg/hyperstack/commit/6267eaeb19e00a3e1c1f76fca417f56170edafb9))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.5 to 0.1.6

## [0.1.5](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.4...hyperstack-interpreter-v0.1.5) (2026-01-09)


### Bug Fixes

* Broken streams after naming refactor ([64437b4](https://github.com/HyperTekOrg/hyperstack/commit/64437b4d80c3b2ec68468ce11bbeaab49678aa8b))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.4 to 0.1.5

## [0.1.4](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.2...hyperstack-interpreter-v0.1.4) (2026-01-09)


### Miscellaneous Chores

* **hyperstack-interpreter:** Synchronize hyperstack versions


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.2 to 0.1.4

## [0.1.2](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.1...hyperstack-interpreter-v0.1.2) (2026-01-09)


### Bug Fixes

* Update naming ([4381946](https://github.com/HyperTekOrg/hyperstack/commit/4381946147e9c51c7de0cb0e63a052c9e9379600))
* Update typescript package name ([6267eae](https://github.com/HyperTekOrg/hyperstack/commit/6267eaeb19e00a3e1c1f76fca417f56170edafb9))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-macros bumped from 0.1.1 to 0.1.2

## [0.1.1](https://github.com/HyperTekOrg/hyperstack/compare/hyperstack-interpreter-v0.1.0...hyperstack-interpreter-v0.1.1) (2026-01-09)


### Features

* Better sdk types during generation ([f9555ef](https://github.com/HyperTekOrg/hyperstack/commit/f9555ef440eb9271a147d178d8b3554cf532b9c7))


### Bug Fixes

* Clippy errors/warnings ([e18fcd6](https://github.com/HyperTekOrg/hyperstack/commit/e18fcd66fb45ee33b0c6019ab65562d286c16eab))
* Naming issues in generated sdk ([179da1f](https://github.com/HyperTekOrg/hyperstack/commit/179da1f2f6c8c75f99c35c0fb90b38576ffc19e2))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * hyperstack-spec-macros bumped from 0.1.0 to 0.1.1

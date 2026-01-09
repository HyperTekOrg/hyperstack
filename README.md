# Hyperstack

Real-time streaming data pipelines for Solana - transform on-chain events into typed state projections.

[![CI](https://github.com/HyperTekOrg/hyperstack/actions/workflows/ci.yml/badge.svg)](https://github.com/HyperTekOrg/hyperstack/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0%2FMIT-blue.svg)](#license)

## Packages

| Package | Language | Registry | Description |
|---------|----------|----------|-------------|
| hyperstack | Rust | crates.io | Umbrella crate re-exporting all components |
| hyperstack-interpreter | Rust | crates.io | AST transformation runtime and VM |
| hyperstack-spec-macros | Rust | crates.io | Proc-macros for stream specifications |
| hyperstack-server | Rust | crates.io | WebSocket server and projection handlers |
| hyperstack-sdk | Rust | crates.io | Rust client SDK |
| hyperstack-cli | Rust | crates.io | CLI tool for SDK generation |
| hyperstack-react | TypeScript | npm | React SDK with hooks |
| hyperstack-sdk | Python | PyPI | Python client SDK |

## Quick Start

### Rust
Add to your `Cargo.toml`:
```toml
[dependencies]
hyperstack = "0.1"
```

### TypeScript / React
```bash
npm install hyperstack-react
```

### Python
```bash
pip install hyperstack-sdk
```

## Repository Structure

- `hyperstack/`: Main umbrella crate
- `interpreter/`: AST transformation runtime and VM
- `spec-macros/`: Proc-macros for stream specifications
- `rust/hyperstack-server/`: WebSocket server and projection handlers
- `rust/hyperstack-sdk/`: Rust client SDK
- `cli/`: CLI tool for SDK generation
- `typescript/`: React SDK with hooks
- `python/hyperstack-sdk/`: Python client SDK

## Releasing

This repo uses [release-please](https://github.com/googleapis/release-please) for automated releases.

### How it works

1. Make commits using [conventional commit](https://www.conventionalcommits.org/) format:
   - `feat: add new feature` - triggers minor version bump
   - `fix: resolve bug` - triggers patch version bump
   - `feat!: breaking change` - triggers major version bump
   - `chore:`, `docs:`, `refactor:` - no version bump

2. Push to `main` - release-please automatically creates/updates a Release PR

3. Merge the Release PR - this:
   - Updates `CHANGELOG.md` in affected packages
   - Bumps versions in `Cargo.toml`, `package.json`, `pyproject.toml`
   - Creates GitHub Releases with component-specific tags
   - Triggers publish workflows to crates.io, npm, and PyPI

### Configuration

| File | Purpose |
|------|---------|
| `release-please-config.json` | Package definitions and release settings |
| `.release-please-manifest.json` | Tracks current version of each package |

### Tag format

Tags follow the pattern `{component}-v{version}`:
- Rust: `hyperstack-v0.2.0`, `hyperstack-cli-v0.2.0`, etc.
- TypeScript: `hyperstack-react-v0.2.0`
- Python: `hyperstack-sdk-python-v0.2.0`

## Development

### Prerequisites

- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Node.js**: 16+ (for TypeScript SDK)
- **Python**: 3.9+ (for Python SDK)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/HyperTekOrg/hyperstack.git
cd hyperstack

# Build all Rust packages
cargo build --workspace

# Build TypeScript SDK
cd typescript && npm install && npm run build

# Install Python SDK in development mode
cd python/hyperstack-sdk && pip install -e .
```

### Running Tests

```bash
# Rust tests
cargo test --workspace

# Rust linting
cargo clippy --workspace -- -D warnings

# TypeScript tests
cd typescript && npm test

# Python tests
cd python/hyperstack-sdk && pytest
```

### Project Structure

```
hyperstack/
├── hyperstack/          # Rust umbrella crate
├── interpreter/         # AST transformation runtime and VM
├── spec-macros/         # Proc-macros for stream specifications
├── cli/                 # CLI tool (hyperstack-cli)
├── rust/
│   ├── hyperstack-sdk/      # Rust client SDK
│   └── hyperstack-server/   # WebSocket server
├── typescript/          # React SDK (hyperstack-react)
├── python/hyperstack-sdk/   # Python client SDK
└── docs/                # Documentation (MDX)
```

## Documentation

- [Concepts Overview](docs/concepts/overview.mdx) - Architecture and core concepts
- [Stack API](docs/concepts/stack-api.mdx) - Client-side API reference
- [CLI Commands](docs/cli/commands.mdx) - CLI usage guide
- [React Quickstart](docs/quickstart/react.mdx) - Getting started with React

## Contributing

We welcome contributions! Here's how to get started:

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Run tests (`cargo test --workspace`)
5. Commit using [conventional commits](https://www.conventionalcommits.org/) format
6. Open a pull request

### Commit Message Format

We use conventional commits for automated releases:

| Prefix | Purpose | Version Bump |
|--------|---------|--------------|
| `feat:` | New feature | Minor |
| `fix:` | Bug fix | Patch |
| `feat!:` or `fix!:` | Breaking change | Major |
| `docs:` | Documentation only | None |
| `chore:` | Maintenance | None |
| `refactor:` | Code refactoring | None |

### Code Style

- **Rust**: Follow `rustfmt` defaults, pass `clippy` with no warnings
- **TypeScript**: Follow ESLint configuration in `typescript/`
- **Python**: Follow PEP 8

## License

This project uses a dual license approach:

- **Rust infrastructure** (hyperstack, interpreter, spec-macros, server, cli): [Apache-2.0](hyperstack/LICENSE)
- **Client SDKs** (TypeScript, Python, Rust SDK): [MIT](typescript/LICENSE)

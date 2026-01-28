# Hyperstack CLI

Programmable data feeds for Solana. Hyperstack CLI lets you deploy feeds for any Solana data to our cloud.

## Quick Start

```bash
npx hyperstack-cli create my-app
cd my-app
npm run dev
```

## Installation

### npm (recommended for JS/TS developers)

```bash
npm install -g hyperstack-cli
```

### Cargo (for Rust developers)

```bash
cargo install hyperstack-cli
```

## Commands

| Command | Description |
|---------|-------------|
| `hyperstack-cli create [name]` | Scaffold a new app from a template |
| `hyperstack-cli init` | Initialize a stack project |
| `hyperstack-cli up` | Deploy your stack |
| `hyperstack-cli status` | Show project overview |
| `hyperstack-cli auth login` | Authenticate with Hyperstack |

Note: If installed via Cargo, the command is `hs` instead of `hyperstack-cli`.

## Documentation

- [Getting Started](https://usehyperstack.com/docs)
- [CLI Reference](https://usehyperstack.com/docs/cli)
- [Stack Development](https://usehyperstack.com/docs/stacks)

## License

Apache-2.0

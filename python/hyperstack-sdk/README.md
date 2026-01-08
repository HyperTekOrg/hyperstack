# hyperstack-sdk

Python SDK for HyperStack - Real-time Solana program data streaming.

## Installation

```bash
pip install hyperstack-sdk
```

## Requirements

- Python 3.9+

## Usage

```python
from hyperstack import HyperstackClient

# Initialize client
client = HyperstackClient()

# Connect and subscribe to program data
await client.connect()
```

## Features

- Real-time WebSocket streaming
- Solana program state subscriptions
- Async/await support

## License

Apache-2.0

## Links

- [Repository](https://github.com/AdrianMAnderson/hyperstack-oss)
- [Issues](https://github.com/AdrianMAnderson/hyperstack-oss/issues)

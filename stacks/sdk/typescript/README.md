# hyperstack-stacks

[![npm](https://img.shields.io/npm/v/hyperstack-stacks.svg)](https://www.npmjs.com/package/hyperstack-stacks)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Protocol stacks for Hyperstack - ready-to-use Solana data streams.

## Installation

```bash
npm install hyperstack-stacks
```

## Usage

### With hyperstack-react

```tsx
import { useHyperstack } from 'hyperstack-react';
import { PUMPFUNTOKEN_STACK } from 'hyperstack-stacks/pumpfun';

function TokenList() {
  const hs = useHyperstack(PUMPFUNTOKEN_STACK);
  const tokens = hs.views.pumpfunToken.list.useWatch();

  return (
    <ul>
      {tokens.map((token) => (
        <li key={token.id?.mint}>
          {token.info?.name} - {token.info?.symbol}
        </li>
      ))}
    </ul>
  );
}
```

### With hyperstack-typescript (framework-agnostic)

```typescript
import { HyperStack } from 'hyperstack-typescript';
import { PUMPFUNTOKEN_STACK, PumpfunToken } from 'hyperstack-stacks/pumpfun';

const hs = await HyperStack.connect('wss://mainnet.hyperstack.xyz', {
  stack: PUMPFUNTOKEN_STACK,
});

// Stream all token updates
for await (const update of hs.views.pumpfunToken.list.watch()) {
  if (update.type === 'upsert') {
    console.log('Token updated:', update.data.info?.name);
  }
}
```

## Available Stacks

### PumpFun Token Stack

Real-time streaming data for PumpFun tokens on Solana.

```typescript
import { PUMPFUNTOKEN_STACK, PumpfunToken } from 'hyperstack-stacks/pumpfun';
```

**Entity: `PumpfunToken`**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `PumpfunTokenId` | Token identifiers (mint, bonding curve) |
| `info` | `PumpfunTokenInfo` | Token metadata (name, symbol, URI) |
| `reserves` | `PumpfunTokenReserves` | Current reserve state and pricing |
| `trading` | `PumpfunTokenTrading` | Trading statistics and metrics |
| `events` | `PumpfunTokenEvents` | Recent buy/sell/create events |

## Peer Dependencies

This package requires one of:

- `hyperstack-react` - For React applications
- `hyperstack-typescript` - For framework-agnostic usage

```bash
# For React
npm install hyperstack-react hyperstack-stacks

# For vanilla TypeScript/JavaScript
npm install hyperstack-typescript hyperstack-stacks
```

## License

MIT

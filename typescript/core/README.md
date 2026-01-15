# Hyperstack TypeScript SDK

Pure TypeScript SDK for the Hyperstack Solana streaming platform. Framework-agnostic core with AsyncIterable-based streaming.

## Installation

```bash
npm install hyperstack-typescript
```

## Usage

### Setup

```typescript
import { HyperStack } from 'hyperstack-typescript';
import { SETTLEMENT_GAME_STACK } from './generated/settlement-game-stack';

const hs = await HyperStack.connect('wss://mainnet.hyperstack.xyz', {
  stack: SETTLEMENT_GAME_STACK,
});
```

### Streaming with AsyncIterable

```typescript
for await (const update of hs.views.settlementGame.list.watch()) {
  if (update.type === 'upsert') {
    console.log('Game updated:', update.key, update.data);
  } else if (update.type === 'delete') {
    console.log('Game deleted:', update.key);
  }
}

for await (const update of hs.views.settlementGame.state.watch('game-123')) {
  console.log('Game 123 updated:', update.data);
}

for await (const update of hs.views.settlementGame.list.watchRich()) {
  if (update.type === 'updated') {
    console.log('Changed from:', update.before);
    console.log('Changed to:', update.after);
  }
}
```

### One-Shot Queries

```typescript
const games = await hs.views.settlementGame.list.get();

const game = await hs.views.settlementGame.state.get('game-123');
```

### Connection Management

```typescript
console.log(hs.connectionState);

const unsubscribe = hs.onConnectionStateChange((state) => {
  console.log('Connection state:', state);
});

await hs.disconnect();
```

## API

### HyperStack

Main client class with typed view accessors.

- `HyperStack.connect(url, options)` - Connect to a HyperStack server
- `views` - Typed view accessors based on your stack definition
- `connectionState` - Current connection state
- `onConnectionStateChange(callback)` - Listen for connection changes
- `disconnect()` - Disconnect from the server

### Views

Each view provides:

**StateView (keyed entities)**
- `watch(key)` - AsyncIterable of updates for a specific key
- `watchRich(key)` - AsyncIterable with before/after diffs
- `get(key)` - Get entity by key
- `getSync(key)` - Get entity synchronously (if available)

**ListView (collections)**
- `watch()` - AsyncIterable of all updates
- `watchRich()` - AsyncIterable with before/after diffs
- `get()` - Get all entities
- `getSync()` - Get all entities synchronously (if available)

### Update Types

```typescript
type Update<T> =
  | { type: 'upsert'; key: string; data: T }
  | { type: 'patch'; key: string; data: Partial<T> }
  | { type: 'delete'; key: string };

type RichUpdate<T> =
  | { type: 'created'; key: string; data: T }
  | { type: 'updated'; key: string; before: T; after: T; patch?: unknown }
  | { type: 'deleted'; key: string; lastKnown?: T };
```

## License

MIT

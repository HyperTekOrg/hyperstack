# Hyperstack React SDK

React SDK for real-time Solana program data streaming from Hyperstack.

Built on top of [`hyperstack-typescript`](https://www.npmjs.com/package/hyperstack-typescript), the pure TypeScript core SDK.

## Installation

```bash
npm install hyperstack-react
```

> **Not using React?** Use [`hyperstack-typescript`](../core/README.md) directly for Vue, Svelte, Node.js, or vanilla JavaScript.

## Usage

### Basic Setup

```tsx
import { HyperstackProvider, useHyperstack, defineStack } from 'hyperstack-react';

const myStack = defineStack({
  // Your stack configuration
});

function App() {
  return (
    <HyperstackProvider config={{ /* your config */ }}>
      <MyComponent />
    </HyperstackProvider>
  );
}

function MyComponent() {
  const stack = useHyperstack(myStack);
  // Use your stack
}
```

### Core Features

- **Real-time Data Streaming**: Subscribe to Solana program state changes
- **React Integration**: Hooks-based API for easy integration with React applications
- **State Management**: Built-in state management with Zustand
- **Type Safety**: Full TypeScript support with comprehensive type definitions
- **View Definitions**: Create state and list views for your data
- **Transaction Handling**: Define and execute transactions with hooks

### API

#### Providers

- `HyperstackProvider` - Root provider for Hyperstack configuration

#### Hooks

- `useHyperstack` - Main hook for accessing stack functionality
- `useHyperstackContext` - Access the runtime context directly

#### Factory Functions

- `defineStack` - Define a new stack configuration
- `createStateView` - Create a state view
- `createListView` - Create a list view
- `createRuntime` - Create a runtime instance

#### Utilities

- `ConnectionManager` - Manage WebSocket connections

## Relationship with hyperstack-typescript

This package depends on and re-exports the core `hyperstack-typescript` package. The core SDK provides:

- `HyperStack` - Main client class
- `ConnectionManager` - WebSocket connection handling
- `EntityStore` - State management
- AsyncIterable-based streaming APIs

The React SDK adds:

- `HyperstackProvider` - React context provider
- `useHyperstack` - Main hook for accessing stacks
- `useConnectionState` - Connection monitoring hook
- `defineStack`, `createStateView`, `createListView` - React-friendly factories

If you need low-level access, you can import directly from the core:

```typescript
import { HyperStack, ConnectionManager } from 'hyperstack-react';
// or
import { HyperStack, ConnectionManager } from 'hyperstack-typescript';
```

## License

MIT

## Author

HyperTek

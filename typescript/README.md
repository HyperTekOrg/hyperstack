# Hyperstack React SDK

TypeScript React SDK for real-time Solana program data streaming from hyperstack

## Installation

```bash
npm install hyperstack-react
```

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

## License

MIT

## Author

HyperTek

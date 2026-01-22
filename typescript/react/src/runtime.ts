import type { StoreApi, UseBoundStore } from 'zustand';
import {
  ConnectionManager,
  FrameProcessor,
  SubscriptionRegistry,
  type Subscription,
  type Frame,
} from 'hyperstack-typescript';
import { Connection as SolanaConnection } from '@solana/web3.js';
import { ZustandAdapter, type HyperStackStore } from './zustand-adapter';
import type { HyperstackConfig, WalletAdapter } from './types';

export interface SubscriptionHandle {
  view: string;
  key?: string;
  filters?: Record<string, string>;
  unsubscribe: () => void;
}

export interface HyperstackRuntime {
  zustandStore: UseBoundStore<StoreApi<HyperStackStore>>;
  adapter: ZustandAdapter;
  connection: ConnectionManager;
  subscriptionRegistry: SubscriptionRegistry;
  wallet?: WalletAdapter;
  solanaConnection?: SolanaConnection;
  subscribe(view: string, key?: string, filters?: Record<string, string>): SubscriptionHandle;
  unsubscribe(handle: SubscriptionHandle): void;
}

export function createRuntime(config: HyperstackConfig & { wallet?: WalletAdapter }): HyperstackRuntime {
  const adapter = new ZustandAdapter();
  const processor = new FrameProcessor(adapter, {
    maxEntriesPerView: config.maxEntriesPerView,
  });

  const connection = new ConnectionManager({
    websocketUrl: config.websocketUrl,
    reconnectIntervals: config.reconnectIntervals,
    maxReconnectAttempts: config.maxReconnectAttempts,
  });

  const subscriptionRegistry = new SubscriptionRegistry(connection);

  connection.onFrame((frame: Frame) => {
    processor.handleFrame(frame);
  });

  connection.onStateChange((state, error) => {
    adapter.setConnectionState(state, error);
  });

  let solanaConnection: SolanaConnection | undefined;
  if (config.connection) {
    solanaConnection = config.connection;
  } else if (config.rpcUrl) {
    solanaConnection = new SolanaConnection(
      config.rpcUrl,
      config.commitment || 'confirmed'
    );
  }

  return {
    zustandStore: adapter.store,
    adapter,
    connection,
    subscriptionRegistry,
    wallet: config.wallet,
    solanaConnection,

    subscribe(view: string, key?: string, filters?: Record<string, string>): SubscriptionHandle {
      const subscription: Subscription = { view, key, filters };
      const unsubscribe = subscriptionRegistry.subscribe(subscription);

      return {
        view,
        key,
        filters,
        unsubscribe,
      };
    },

    unsubscribe(handle: SubscriptionHandle) {
      handle.unsubscribe();
    },
  };
}

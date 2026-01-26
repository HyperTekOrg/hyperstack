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
import { DEFAULT_FLUSH_INTERVAL_MS, type HyperstackConfig, type WalletAdapter } from './types';

export interface SubscriptionHandle {
  view: string;
  key?: string;
  filters?: Record<string, string>;
  take?: number;
  skip?: number;
  unsubscribe: () => void;
}

export interface HyperstackRuntime {
  zustandStore: UseBoundStore<StoreApi<HyperStackStore>>;
  adapter: ZustandAdapter;
  connection: ConnectionManager;
  subscriptionRegistry: SubscriptionRegistry;
  wallet?: WalletAdapter;
  solanaConnection?: SolanaConnection;
  subscribe(view: string, key?: string, filters?: Record<string, string>, take?: number, skip?: number): SubscriptionHandle;
  unsubscribe(handle: SubscriptionHandle): void;
}

export function createRuntime(config: HyperstackConfig & { wallet?: WalletAdapter }): HyperstackRuntime {
  const adapter = new ZustandAdapter();
  const processor = new FrameProcessor(adapter, {
    maxEntriesPerView: config.maxEntriesPerView,
    flushIntervalMs: config.flushIntervalMs ?? DEFAULT_FLUSH_INTERVAL_MS,
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

    subscribe(view: string, key?: string, filters?: Record<string, string>, take?: number, skip?: number): SubscriptionHandle {
      const subscription: Subscription = { view, key, filters, take, skip };
      const unsubscribe = subscriptionRegistry.subscribe(subscription);

      return {
        view,
        key,
        filters,
        take,
        skip,
        unsubscribe,
      };
    },

    unsubscribe(handle: SubscriptionHandle) {
      handle.unsubscribe();
    },
  };
}

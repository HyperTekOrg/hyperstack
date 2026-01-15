import { ConnectionManager } from './connection';
import { createHyperStore } from './store';
import { HyperstackConfig, Subscription, WalletAdapter } from './types';

export interface SubscriptionHandle {
  view: string;
  key?: string;
  filters?: Record<string, string>;
  unsubscribe: () => void;
}

export interface HyperstackRuntime {
  store: ReturnType<typeof createHyperStore>;
  connection: ConnectionManager;
  wallet?: WalletAdapter;
  subscribe(view: string, key?: string, filters?: Record<string, string>): SubscriptionHandle;
  unsubscribe(handle: SubscriptionHandle): void;
}

export function createRuntime(config: HyperstackConfig): HyperstackRuntime {
  const store = createHyperStore({
    websocketUrl: config.websocketUrl
  });
  const connection = store.getState().connectionManager;

  return {
    store,
    connection,
    wallet: config.wallet,
    subscribe(view: string, key?: string, filters?: Record<string, string>): SubscriptionHandle {
      const subscription: Subscription = { view, key, filters };
      store.getState()._incRef(subscription);

      return {
        view,
        key,
        filters,
        unsubscribe: () => store.getState()._decRef(subscription)
      };
    },
    unsubscribe(handle: SubscriptionHandle) {
      handle.unsubscribe();
    }
  };
}

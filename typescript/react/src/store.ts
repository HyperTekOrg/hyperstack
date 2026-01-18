import { create } from 'zustand';
import { ConnectionManager } from './connection';
import {
  ConnectionState,
  EntityFrame,
  Frame,
  Subscription,
  HyperState,
  DEFAULT_CONFIG,
  HyperSDKConfig,
  ViewMode,
  isSnapshotFrame
} from './types';

function deepMerge<T>(target: T, source: Partial<T>): T {
  if (!isObject(target) || !isObject(source)) {
    return source as T;
  }

  const result = { ...target };

  for (const key in source) {
    const sourceValue = source[key];
    const targetValue = result[key];

    if (isObject(sourceValue) && isObject(targetValue)) {
      result[key] = deepMerge(targetValue, sourceValue as any);
    } else {
      result[key] = sourceValue as any;
    }
  }

  return result;
}

function isObject(item: any): item is Record<string, any> {
  return item && typeof item === 'object' && !Array.isArray(item);
}

// Subscription key for tracking ref counts across components
type SubKey = string; // Format: `${view}:${key ?? '*'}:${partition ?? ''}:${JSON.stringify(filters ?? {})}`

// Internal tracking for subscription reference counting (deduplicates network subscriptions (?))
interface SubscriptionTracker {
  subscription: Subscription;  // the actual subscription details sent to server
  refCount: number;            // how many components are using this subscription
}

interface ViewMetadata {
  mode: ViewMode;
  keys: string[];
  lastArgs?: any;
  lastUpdatedAt?: number;
}

interface ViewCache {
  [viewPath: string]: ViewMetadata;
}

// The complete Zustand store interface - extends HyperState with actions
interface HyperStore extends HyperState {
  // core frame processor - processes incoming WebSocket frames
  handleFrame: <T>(frame: Frame<T>) => void;  // core frame processor (called by ConnectionManager)

  // Subscription lifecycle with automatic ref counting
  // Multiple components can request same data, only 1 network subscription happens
  _incRef: (subscription: Subscription) => void;
  _decRef: (subscription: Subscription) => void;
  _getRefCount: (subscription: Subscription) => number;

  // manual subscription
  subscribe: (subscription: Subscription) => void;      // immediate subscription, no ref counting
  unsubscribe: (entity: string, key?: string) => void;  // immediate unsubscribe, no ref counting

  // connection management (delegated to ConnectionManager)
  connect: () => void;                                     // initiate WebSocket connection
  disconnect: () => void;                                  // close WebSocket connection
  updateConfig: (config: Partial<HyperSDKConfig>) => void; // update connection config

  // internal state tracking
  subscriptionRefs: Map<SubKey, SubscriptionTracker>; // the "phone book" of active subscriptions, maps subscription identifiers → { subscription details, how many components are using it }
  connectionManager: ConnectionManager;               // WebSocket connection instance
  viewCache: ViewCache;                               // metadata tracking per view
}

export function createHyperStore(config: Partial<HyperSDKConfig> = {}) {
  return create<HyperStore>((set, get) => {
    const connectionManager = new ConnectionManager({ ...DEFAULT_CONFIG, ...config });

    const makeSubKey = (subscription: Subscription): SubKey =>
      `${subscription.view}:${subscription.key ?? '*'}:${subscription.partition ?? ''}:${JSON.stringify(subscription.filters ?? {})}`;

    connectionManager.setHandlers({
      onFrame: <T>(frame: Frame<T>) => {
        get().handleFrame(frame);
      },

      onStateChange: (connectionState: ConnectionState, error?: string) => {
        set({ connectionState, lastError: error });
      }
    });

    return {
      connectionState: 'disconnected' as ConnectionState,
      lastError: undefined,
      entities: new Map(),
      recentFrames: [],
      subscriptionRefs: new Map(),
      connectionManager,
      viewCache: {},

      handleFrame: <T>(frame: Frame<T>) => {
        set((state) => {
          const newEntities = new Map(state.entities);
          const viewPath = frame.entity;
          const entityMap = new Map(newEntities.get(viewPath) || new Map());

          if (isSnapshotFrame(frame)) {
            for (const entity of frame.data) {
              entityMap.set(entity.key, entity.data);
            }
          } else {
            switch (frame.op) {
              case 'upsert':
              case 'create':
                entityMap.set(frame.key, frame.data);
                break;
              case 'patch':
                const existing = entityMap.get(frame.key);
                if (existing && typeof existing === 'object' && typeof frame.data === 'object') {
                  entityMap.set(frame.key, deepMerge(existing, frame.data as any));
                } else {
                  entityMap.set(frame.key, frame.data);
                }
                break;
              case 'delete':
                entityMap.delete(frame.key);
                break;
            }
          }

          newEntities.set(viewPath, entityMap);

          const newViewCache = { ...state.viewCache };
          const currentMetadata = newViewCache[viewPath];

          const keys = Array.from(entityMap.keys());
          newViewCache[viewPath] = {
            mode: (frame.mode === 'state' || frame.mode === 'list') ? frame.mode : 'list',
            keys,
            lastUpdatedAt: Date.now(),
            lastArgs: currentMetadata?.lastArgs
          };

          return {
            ...state,
            entities: newEntities,
            recentFrames: [frame as EntityFrame<T>, ...state.recentFrames],
            viewCache: newViewCache
          };
        });
      },

      _incRef: (subscription: Subscription) => {
        const subKey = makeSubKey(subscription);
        const { subscriptionRefs } = get();

        const existing = subscriptionRefs.get(subKey);
        if (existing) {
          existing.refCount++;
        } else {
          subscriptionRefs.set(subKey, {
            subscription,
            refCount: 1
          });

          connectionManager.subscribe(subscription);
        }
      },

      _decRef: (subscription: Subscription) => {
        const subKey = makeSubKey(subscription);
        const { subscriptionRefs } = get();

        const existing = subscriptionRefs.get(subKey);
        if (existing) {
          existing.refCount--;
          if (existing.refCount <= 0) {
            subscriptionRefs.delete(subKey);

            connectionManager.unsubscribe(subscription.view, subscription.key);
          }
        }
      },

      _getRefCount: (subscription: Subscription) => {
        const subKey = makeSubKey(subscription);
        return get().subscriptionRefs.get(subKey)?.refCount ?? 0;
      },

      subscribe: (subscription: Subscription) => {
        connectionManager.subscribe(subscription);
      },

      unsubscribe: (view: string, key?: string) => {
        connectionManager.unsubscribe(view, key);
      },

      connect: () => {
        connectionManager.connect();
      },

      disconnect: () => {
        connectionManager.disconnect();
      },

      updateConfig: (newConfig: Partial<HyperSDKConfig>) => {
        connectionManager.updateConfig(newConfig);
      }
    };
  });
}

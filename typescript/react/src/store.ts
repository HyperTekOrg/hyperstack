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
  isSnapshotFrame,
  DEFAULT_MAX_ENTRIES_PER_VIEW,
} from './types';

class ViewData<T = unknown> {
  private entities: Map<string, T> = new Map();
  private accessOrder: string[] = [];

  get(key: string): T | undefined {
    return this.entities.get(key);
  }

  set(key: string, value: T): void {
    if (!this.entities.has(key)) {
      this.accessOrder.push(key);
    } else {
      this.touch(key);
    }
    this.entities.set(key, value);
  }

  delete(key: string): boolean {
    const idx = this.accessOrder.indexOf(key);
    if (idx !== -1) {
      this.accessOrder.splice(idx, 1);
    }
    return this.entities.delete(key);
  }

  has(key: string): boolean {
    return this.entities.has(key);
  }

  values(): IterableIterator<T> {
    return this.entities.values();
  }

  keys(): string[] {
    return Array.from(this.entities.keys());
  }

  get size(): number {
    return this.entities.size;
  }

  touch(key: string): void {
    const idx = this.accessOrder.indexOf(key);
    if (idx !== -1) {
      this.accessOrder.splice(idx, 1);
      this.accessOrder.push(key);
    }
  }

  evictOldest(): string | undefined {
    const oldest = this.accessOrder.shift();
    if (oldest !== undefined) {
      this.entities.delete(oldest);
    }
    return oldest;
  }

  toMap(): Map<string, T> {
    return new Map(this.entities);
  }
}

function isObject(item: any): item is Record<string, any> {
  return item && typeof item === 'object' && !Array.isArray(item);
}

function deepMergeWithAppend<T>(
  target: T,
  source: Partial<T>,
  appendPaths: string[],
  currentPath = ''
): T {
  if (!isObject(target) || !isObject(source)) {
    return source as T;
  }

  const result = { ...target };

  for (const key in source) {
    const sourceValue = source[key];
    const targetValue = result[key];
    const fieldPath = currentPath ? `${currentPath}.${key}` : key;

    if (Array.isArray(sourceValue) && Array.isArray(targetValue)) {
      if (appendPaths.includes(fieldPath)) {
        result[key] = [...targetValue, ...sourceValue] as any;
      } else {
        result[key] = sourceValue as any;
      }
    } else if (isObject(sourceValue) && isObject(targetValue)) {
      result[key] = deepMergeWithAppend(
        targetValue,
        sourceValue as any,
        appendPaths,
        fieldPath
      );
    } else {
      result[key] = sourceValue as any;
    }
  }

  return result;
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
  viewDataMap: Map<string, ViewData<unknown>>;        // LRU-tracked view data
  maxEntriesPerView: number | null;                   // max entries before eviction
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

    const maxEntriesPerView = config.maxEntriesPerView === undefined
      ? DEFAULT_MAX_ENTRIES_PER_VIEW
      : config.maxEntriesPerView;

    const enforceMaxEntries = (viewData: ViewData<unknown>, max: number | null) => {
      if (max === null) return;
      while (viewData.size > max) {
        viewData.evictOldest();
      }
    };

    return {
      connectionState: 'disconnected' as ConnectionState,
      lastError: undefined,
      entities: new Map(),
      recentFrames: [],
      subscriptionRefs: new Map(),
      connectionManager,
      viewCache: {},
      viewDataMap: new Map(),
      maxEntriesPerView,

      handleFrame: <T>(frame: Frame<T>) => {
        set((state) => {
          const newViewDataMap = new Map(state.viewDataMap);
          const viewPath = frame.entity;
          let viewData = newViewDataMap.get(viewPath);
          if (!viewData) {
            viewData = new ViewData();
            newViewDataMap.set(viewPath, viewData);
          }

          if (isSnapshotFrame(frame)) {
            for (const entity of frame.data) {
              viewData.set(entity.key, entity.data);
            }
            enforceMaxEntries(viewData, state.maxEntriesPerView);
          } else {
            switch (frame.op) {
              case 'upsert':
              case 'create':
                viewData.set(frame.key, frame.data);
                enforceMaxEntries(viewData, state.maxEntriesPerView);
                break;
              case 'patch':
                const existing = viewData.get(frame.key);
                const appendPaths = frame.append ?? [];
                if (existing && typeof existing === 'object' && typeof frame.data === 'object') {
                  viewData.set(
                    frame.key,
                    deepMergeWithAppend(existing, frame.data as any, appendPaths)
                  );
                } else {
                  viewData.set(frame.key, frame.data);
                }
                enforceMaxEntries(viewData, state.maxEntriesPerView);
                break;
              case 'delete':
                viewData.delete(frame.key);
                break;
            }
          }

          const newEntities = new Map(state.entities);
          newEntities.set(viewPath, viewData.toMap());

          const newViewCache = { ...state.viewCache };
          const currentMetadata = newViewCache[viewPath];

          const keys = viewData.keys();
          newViewCache[viewPath] = {
            mode: (frame.mode === 'state' || frame.mode === 'list') ? frame.mode : 'list',
            keys,
            lastUpdatedAt: Date.now(),
            lastArgs: currentMetadata?.lastArgs
          };

          return {
            ...state,
            entities: newEntities,
            viewDataMap: newViewDataMap,
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

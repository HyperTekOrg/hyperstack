import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type {
  StorageAdapter,
  UpdateCallback,
  RichUpdateCallback,
  StorageAdapterConfig,
  ViewSortConfig,
  Update,
  RichUpdate,
  ConnectionState,
  SortConfig,
} from 'hyperstack-typescript';

interface ZustandState {
  entities: Map<string, Map<string, unknown>>;
  sortedKeys: Map<string, string[]>;
  viewConfigs: Map<string, ViewSortConfig>;
  connectionState: ConnectionState;
  lastError?: string;
}

interface ZustandActions {
  _set: <T>(viewPath: string, key: string, data: T) => void;
  _delete: (viewPath: string, key: string) => void;
  _clear: (viewPath?: string) => void;
  _setConnectionState: (state: ConnectionState, error?: string) => void;
  _setViewConfig: (viewPath: string, config: ViewSortConfig) => void;
  _updateSortedKeys: (viewPath: string, sortedKeys: string[]) => void;
}

export type HyperStackStore = ZustandState & ZustandActions;

function getNestedValue(obj: unknown, path: string[]): unknown {
  let current: unknown = obj;
  for (const segment of path) {
    if (current === null || current === undefined) return undefined;
    if (typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

function compareSortValues(a: unknown, b: unknown): number {
  if (a === b) return 0;
  if (a === undefined || a === null) return -1;
  if (b === undefined || b === null) return 1;

  if (typeof a === 'number' && typeof b === 'number') {
    return a - b;
  }
  if (typeof a === 'string' && typeof b === 'string') {
    return a.localeCompare(b);
  }
  if (typeof a === 'boolean' && typeof b === 'boolean') {
    return (a ? 1 : 0) - (b ? 1 : 0);
  }

  return String(a).localeCompare(String(b));
}

function binarySearchInsertPosition(
  sortedKeys: string[],
  entities: Map<string, unknown>,
  sortConfig: SortConfig,
  newKey: string,
  newValue: unknown
): number {
  const newSortValue = getNestedValue(newValue, sortConfig.field);
  const isDesc = sortConfig.order === 'desc';
  let low = 0;
  let high = sortedKeys.length;

  while (low < high) {
    const mid = Math.floor((low + high) / 2);
    const midKey = sortedKeys[mid];
    const midEntity = entities.get(midKey);
    const midValue = getNestedValue(midEntity, sortConfig.field);

    let cmp = compareSortValues(newSortValue, midValue);
    if (isDesc) cmp = -cmp;

    if (cmp === 0) {
      cmp = newKey.localeCompare(midKey);
    }

    if (cmp < 0) {
      high = mid;
    } else {
      low = mid + 1;
    }
  }

  return low;
}

export class ZustandAdapter implements StorageAdapter {
  private updateCallbacks: Set<UpdateCallback> = new Set();
  private richUpdateCallbacks: Set<RichUpdateCallback> = new Set();
  private accessOrder: Map<string, string[]> = new Map();

  public readonly store: UseBoundStore<StoreApi<HyperStackStore>>;

  constructor(_config: StorageAdapterConfig = {}) {
    this.store = create<HyperStackStore>((set) => ({
      entities: new Map(),
      sortedKeys: new Map(),
      viewConfigs: new Map(),
      connectionState: 'disconnected',
      lastError: undefined,

      _set: <T>(viewPath: string, key: string, data: T) => {
        set((state) => {
          const newEntities = new Map(state.entities);
          const viewMap = new Map(newEntities.get(viewPath) ?? new Map());
          viewMap.set(key, data);
          newEntities.set(viewPath, viewMap);
          return { entities: newEntities };
        });
      },

      _delete: (viewPath: string, key: string) => {
        set((state) => {
          const newEntities = new Map(state.entities);
          const viewMap = newEntities.get(viewPath);
          if (viewMap) {
            const newViewMap = new Map(viewMap);
            newViewMap.delete(key);
            newEntities.set(viewPath, newViewMap);
          }
          return { entities: newEntities };
        });
      },

      _clear: (viewPath?: string) => {
        set((state) => {
          if (viewPath) {
            const newEntities = new Map(state.entities);
            newEntities.delete(viewPath);
            const newSortedKeys = new Map(state.sortedKeys);
            newSortedKeys.delete(viewPath);
            return { entities: newEntities, sortedKeys: newSortedKeys };
          }
          return { entities: new Map(), sortedKeys: new Map() };
        });
      },

      _setConnectionState: (connectionState, lastError) => {
        set({ connectionState, lastError });
      },

      _setViewConfig: (viewPath: string, config: ViewSortConfig) => {
        set((state) => {
          const newConfigs = new Map(state.viewConfigs);
          newConfigs.set(viewPath, config);
          return { viewConfigs: newConfigs };
        });
      },

      _updateSortedKeys: (viewPath: string, newSortedKeys: string[]) => {
        set((state) => {
          const newSortedKeysMap = new Map(state.sortedKeys);
          newSortedKeysMap.set(viewPath, newSortedKeys);
          return { sortedKeys: newSortedKeysMap };
        });
      },
    }));
  }

  get<T>(viewPath: string, key: string): T | null {
    const entities = this.store.getState().entities;
    const viewMap = entities.get(viewPath);
    if (!viewMap) return null;
    const value = viewMap.get(key);
    return value !== undefined ? (value as T) : null;
  }

  getAll<T>(viewPath: string): T[] {
    const state = this.store.getState();
    const viewMap = state.entities.get(viewPath);
    if (!viewMap) return [];

    const sortedKeys = state.sortedKeys.get(viewPath);
    if (sortedKeys && sortedKeys.length > 0) {
      return sortedKeys.map(k => viewMap.get(k)).filter(v => v !== undefined) as T[];
    }

    return Array.from(viewMap.values()) as T[];
  }

  getAllSync<T>(viewPath: string): T[] | undefined {
    const state = this.store.getState();
    const viewMap = state.entities.get(viewPath);
    if (!viewMap) return undefined;

    const sortedKeys = state.sortedKeys.get(viewPath);
    if (sortedKeys && sortedKeys.length > 0) {
      return sortedKeys.map(k => viewMap.get(k)).filter(v => v !== undefined) as T[];
    }

    return Array.from(viewMap.values()) as T[];
  }

  getSync<T>(viewPath: string, key: string): T | null | undefined {
    const entities = this.store.getState().entities;
    const viewMap = entities.get(viewPath);
    if (!viewMap) return undefined;
    const value = viewMap.get(key);
    return value !== undefined ? (value as T) : null;
  }

  has(viewPath: string, key: string): boolean {
    return this.store.getState().entities.get(viewPath)?.has(key) ?? false;
  }

  keys(viewPath: string): string[] {
    const viewMap = this.store.getState().entities.get(viewPath);
    if (!viewMap) return [];
    return Array.from(viewMap.keys());
  }

  size(viewPath: string): number {
    return this.store.getState().entities.get(viewPath)?.size ?? 0;
  }

  set<T>(viewPath: string, key: string, data: T): void {
    const state = this.store.getState();
    const viewConfig = state.viewConfigs.get(viewPath);

    if (viewConfig?.sort) {
      const viewMap = state.entities.get(viewPath) ?? new Map();
      const currentSortedKeys = [...(state.sortedKeys.get(viewPath) ?? [])];
      
      const existingIdx = currentSortedKeys.indexOf(key);
      if (existingIdx !== -1) {
        currentSortedKeys.splice(existingIdx, 1);
      }

      const tempMap = new Map(viewMap);
      tempMap.set(key, data);

      const insertIdx = binarySearchInsertPosition(
        currentSortedKeys,
        tempMap,
        viewConfig.sort,
        key,
        data
      );
      currentSortedKeys.splice(insertIdx, 0, key);

      state._set(viewPath, key, data);
      state._updateSortedKeys(viewPath, currentSortedKeys);
    } else {
      let order = this.accessOrder.get(viewPath);
      if (!order) {
        order = [];
        this.accessOrder.set(viewPath, order);
      }

      const existingIdx = order.indexOf(key);
      if (existingIdx !== -1) {
        order.splice(existingIdx, 1);
      }
      order.push(key);

      state._set(viewPath, key, data);
    }
  }

  delete(viewPath: string, key: string): void {
    const state = this.store.getState();
    const viewConfig = state.viewConfigs.get(viewPath);

    if (viewConfig?.sort) {
      const currentSortedKeys = state.sortedKeys.get(viewPath);
      if (currentSortedKeys) {
        const newSortedKeys = currentSortedKeys.filter(k => k !== key);
        state._updateSortedKeys(viewPath, newSortedKeys);
      }
    } else {
      const order = this.accessOrder.get(viewPath);
      if (order) {
        const idx = order.indexOf(key);
        if (idx !== -1) {
          order.splice(idx, 1);
        }
      }
    }

    state._delete(viewPath, key);
  }

  clear(viewPath?: string): void {
    if (viewPath) {
      this.accessOrder.delete(viewPath);
    } else {
      this.accessOrder.clear();
    }

    this.store.getState()._clear(viewPath);
  }

  evictOldest(viewPath: string): string | undefined {
    const order = this.accessOrder.get(viewPath);
    if (!order || order.length === 0) return undefined;

    const oldest = order.shift();
    if (oldest !== undefined) {
      this.store.getState()._delete(viewPath, oldest);
    }
    return oldest;
  }

  onUpdate(callback: UpdateCallback): () => void {
    this.updateCallbacks.add(callback);
    return () => this.updateCallbacks.delete(callback);
  }

  onRichUpdate(callback: RichUpdateCallback): () => void {
    this.richUpdateCallbacks.add(callback);
    return () => this.richUpdateCallbacks.delete(callback);
  }

  notifyUpdate<T>(viewPath: string, key: string, update: Update<T>): void {
    for (const callback of this.updateCallbacks) {
      callback(viewPath, key, update);
    }
  }

  notifyRichUpdate<T>(viewPath: string, key: string, update: RichUpdate<T>): void {
    for (const callback of this.richUpdateCallbacks) {
      callback(viewPath, key, update);
    }
  }

  setConnectionState(state: ConnectionState, error?: string): void {
    this.store.getState()._setConnectionState(state, error);
  }

  setViewConfig(viewPath: string, config: ViewSortConfig): void {
    const state = this.store.getState();
    const existingConfig = state.viewConfigs.get(viewPath);
    if (existingConfig?.sort) return;

    state._setViewConfig(viewPath, config);

    if (config.sort) {
      this.rebuildSortedKeys(viewPath, config.sort);
    }
  }

  getViewConfig(viewPath: string): ViewSortConfig | undefined {
    return this.store.getState().viewConfigs.get(viewPath);
  }

  private rebuildSortedKeys(viewPath: string, sortConfig: SortConfig): void {
    const state = this.store.getState();
    const viewMap = state.entities.get(viewPath);
    if (!viewMap || viewMap.size === 0) return;

    const isDesc = sortConfig.order === 'desc';
    const entries = Array.from(viewMap.entries());

    entries.sort((a, b) => {
      const aValue = getNestedValue(a[1], sortConfig.field);
      const bValue = getNestedValue(b[1], sortConfig.field);
      let cmp = compareSortValues(aValue, bValue);
      if (isDesc) cmp = -cmp;
      if (cmp === 0) {
        cmp = a[0].localeCompare(b[0]);
      }
      return cmp;
    });

    const sortedKeys = entries.map(([k]) => k);
    state._updateSortedKeys(viewPath, sortedKeys);
  }
}

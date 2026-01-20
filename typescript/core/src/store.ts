import type { EntityFrame, SnapshotFrame, Frame } from './frame';
import { isSnapshotFrame } from './frame';
import type { Update, RichUpdate, SubscribeCallback, UnsubscribeFn } from './types';
import { DEFAULT_MAX_ENTRIES_PER_VIEW } from './types';

export interface EntityStoreConfig {
  maxEntriesPerView?: number | null;
}

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

  keys(): IterableIterator<string> {
    return this.entities.keys();
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
}

function isObject(item: unknown): item is Record<string, unknown> {
  return item !== null && typeof item === 'object' && !Array.isArray(item);
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

  const result = { ...target } as Record<string, unknown>;

  for (const key in source) {
    const sourceValue = source[key];
    const targetValue = result[key];
    const fieldPath = currentPath ? `${currentPath}.${key}` : key;

    if (Array.isArray(sourceValue) && Array.isArray(targetValue)) {
      if (appendPaths.includes(fieldPath)) {
        result[key] = [...targetValue, ...sourceValue];
      } else {
        result[key] = sourceValue;
      }
    } else if (isObject(sourceValue) && isObject(targetValue)) {
      result[key] = deepMergeWithAppend(
        targetValue,
        sourceValue as Record<string, unknown>,
        appendPaths,
        fieldPath
      );
    } else {
      result[key] = sourceValue;
    }
  }

  return result as T;
}

type EntityUpdateCallback = (viewPath: string, key: string, update: Update<unknown>) => void;
type RichUpdateCallback = (viewPath: string, key: string, update: RichUpdate<unknown>) => void;

export class EntityStore {
  private views: Map<string, ViewData<unknown>> = new Map();
  private updateCallbacks: Set<EntityUpdateCallback> = new Set();
  private richUpdateCallbacks: Set<RichUpdateCallback> = new Set();
  private maxEntriesPerView: number | null;

  constructor(config: EntityStoreConfig = {}) {
    this.maxEntriesPerView = config.maxEntriesPerView === undefined
      ? DEFAULT_MAX_ENTRIES_PER_VIEW
      : config.maxEntriesPerView;
  }

  private enforceMaxEntries(viewData: ViewData<unknown>): void {
    if (this.maxEntriesPerView === null) return;
    while (viewData.size > this.maxEntriesPerView) {
      viewData.evictOldest();
    }
  }

  handleFrame<T>(frame: Frame<T>): void {
    console.log('[hyperstack] handleFrame called:', { op: frame.op, entity: frame.entity, isSnapshot: isSnapshotFrame(frame) });
    if (isSnapshotFrame(frame)) {
      this.handleSnapshotFrame(frame);
      return;
    }
    this.handleEntityFrame(frame);
  }

  private handleSnapshotFrame<T>(frame: SnapshotFrame<T>): void {
    const viewPath = frame.entity;
    console.log('[hyperstack] handleSnapshotFrame:', { viewPath, entityCount: frame.data.length });
    let viewData = this.views.get(viewPath);

    if (!viewData) {
      viewData = new ViewData();
      this.views.set(viewPath, viewData);
    }

    for (const entity of frame.data) {
      const previousValue = viewData.get(entity.key) as T | undefined;
      viewData.set(entity.key, entity.data);
      this.notifyUpdate(viewPath, entity.key, {
        type: 'upsert',
        key: entity.key,
        data: entity.data,
      });
      this.notifyRichUpdate(viewPath, entity.key, previousValue, entity.data, 'upsert');
    }
    this.enforceMaxEntries(viewData);
    console.log('[hyperstack] handleSnapshotFrame complete, viewData size:', viewData.size);
  }

  private handleEntityFrame<T>(frame: EntityFrame<T>): void {
    const viewPath = frame.entity;
    let viewData = this.views.get(viewPath);

    if (!viewData) {
      viewData = new ViewData();
      this.views.set(viewPath, viewData);
    }

    const previousValue = viewData.get(frame.key) as T | undefined;

    switch (frame.op) {
      case 'create':
      case 'upsert':
        viewData.set(frame.key, frame.data);
        this.enforceMaxEntries(viewData);
        this.notifyUpdate(viewPath, frame.key, {
          type: 'upsert',
          key: frame.key,
          data: frame.data,
        });
        this.notifyRichUpdate(viewPath, frame.key, previousValue, frame.data, frame.op);
        break;

      case 'patch': {
        const existing = viewData.get(frame.key);
        const appendPaths = frame.append ?? [];
        const merged = existing
          ? deepMergeWithAppend(existing, frame.data as Partial<unknown>, appendPaths)
          : frame.data;
        viewData.set(frame.key, merged);
        this.enforceMaxEntries(viewData);
        this.notifyUpdate(viewPath, frame.key, {
          type: 'patch',
          key: frame.key,
          data: frame.data as Partial<unknown>,
        });
        this.notifyRichUpdate(viewPath, frame.key, previousValue, merged as T, 'patch', frame.data);
        break;
      }

      case 'delete':
        viewData.delete(frame.key);
        this.notifyUpdate(viewPath, frame.key, {
          type: 'delete',
          key: frame.key,
        });
        if (previousValue !== undefined) {
          this.notifyRichDelete(viewPath, frame.key, previousValue);
        }
        break;
    }
  }

  getAll<T>(viewPath: string): T[] {
    const viewData = this.views.get(viewPath);
    if (!viewData) return [];
    return Array.from(viewData.values()) as T[];
  }

  get<T>(viewPath: string, key: string): T | null {
    const viewData = this.views.get(viewPath);
    if (!viewData) return null;
    const value = viewData.get(key);
    return value !== undefined ? (value as T) : null;
  }

  getAllSync<T>(viewPath: string): T[] | undefined {
    const viewData = this.views.get(viewPath);
    if (!viewData) return undefined;
    return Array.from(viewData.values()) as T[];
  }

  getSync<T>(viewPath: string, key: string): T | null | undefined {
    const viewData = this.views.get(viewPath);
    if (!viewData) return undefined;
    const value = viewData.get(key);
    return value !== undefined ? (value as T) : null;
  }

  keys(viewPath: string): string[] {
    const viewData = this.views.get(viewPath);
    if (!viewData) return [];
    return Array.from(viewData.keys());
  }

  size(viewPath: string): number {
    const viewData = this.views.get(viewPath);
    return viewData?.size ?? 0;
  }

  clear(): void {
    this.views.clear();
  }

  clearView(viewPath: string): void {
    this.views.delete(viewPath);
  }

  onUpdate(callback: EntityUpdateCallback): UnsubscribeFn {
    this.updateCallbacks.add(callback);
    return () => {
      this.updateCallbacks.delete(callback);
    };
  }

  onRichUpdate(callback: RichUpdateCallback): UnsubscribeFn {
    this.richUpdateCallbacks.add(callback);
    return () => {
      this.richUpdateCallbacks.delete(callback);
    };
  }

  subscribe<T>(viewPath: string, callback: SubscribeCallback<T>): UnsubscribeFn {
    const handler: EntityUpdateCallback = (path, _key, update) => {
      if (path === viewPath) {
        callback(update as Update<T>);
      }
    };
    this.updateCallbacks.add(handler);
    return () => {
      this.updateCallbacks.delete(handler);
    };
  }

  subscribeToKey<T>(
    viewPath: string,
    key: string,
    callback: SubscribeCallback<T>
  ): UnsubscribeFn {
    const handler: EntityUpdateCallback = (path, updateKey, update) => {
      if (path === viewPath && updateKey === key) {
        callback(update as Update<T>);
      }
    };
    this.updateCallbacks.add(handler);
    return () => {
      this.updateCallbacks.delete(handler);
    };
  }

  private notifyUpdate(viewPath: string, key: string, update: Update<unknown>): void {
    for (const callback of this.updateCallbacks) {
      callback(viewPath, key, update);
    }
  }

  private notifyRichUpdate<T>(
    viewPath: string,
    key: string,
    before: T | undefined,
    after: T,
    _op: 'create' | 'upsert' | 'patch',
    patch?: unknown
  ): void {
    const richUpdate: RichUpdate<T> =
      before === undefined
        ? { type: 'created', key, data: after }
        : { type: 'updated', key, before, after, patch };

    for (const callback of this.richUpdateCallbacks) {
      callback(viewPath, key, richUpdate as RichUpdate<unknown>);
    }
  }

  private notifyRichDelete<T>(viewPath: string, key: string, lastKnown: T): void {
    const richUpdate: RichUpdate<T> = { type: 'deleted', key, lastKnown };

    for (const callback of this.richUpdateCallbacks) {
      callback(viewPath, key, richUpdate as RichUpdate<unknown>);
    }
  }
}

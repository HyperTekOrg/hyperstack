import type { EntityFrame, SnapshotFrame, Frame } from './frame';
import { isSnapshotFrame } from './frame';
import type { Update, RichUpdate, SubscribeCallback, UnsubscribeFn } from './types';

function isObject(item: unknown): item is Record<string, unknown> {
  return item !== null && typeof item === 'object' && !Array.isArray(item);
}

function deepMerge<T>(target: T, source: Partial<T>): T {
  if (!isObject(target) || !isObject(source)) {
    return source as T;
  }

  const result = { ...target } as Record<string, unknown>;

  for (const key in source) {
    const sourceValue = source[key];
    const targetValue = result[key];

    if (isObject(sourceValue) && isObject(targetValue)) {
      result[key] = deepMerge(targetValue, sourceValue as Record<string, unknown>);
    } else {
      result[key] = sourceValue;
    }
  }

  return result as T;
}

type EntityUpdateCallback = (viewPath: string, key: string, update: Update<unknown>) => void;
type RichUpdateCallback = (viewPath: string, key: string, update: RichUpdate<unknown>) => void;

export class EntityStore {
  private entities: Map<string, Map<string, unknown>> = new Map();
  private updateCallbacks: Set<EntityUpdateCallback> = new Set();
  private richUpdateCallbacks: Set<RichUpdateCallback> = new Set();

  handleFrame<T>(frame: Frame<T>): void {
    if (isSnapshotFrame(frame)) {
      this.handleSnapshotFrame(frame);
      return;
    }
    this.handleEntityFrame(frame);
  }

  private handleSnapshotFrame<T>(frame: SnapshotFrame<T>): void {
    const viewPath = frame.entity;
    let viewMap = this.entities.get(viewPath);

    if (!viewMap) {
      viewMap = new Map();
      this.entities.set(viewPath, viewMap);
    }

    for (const entity of frame.data) {
      const previousValue = viewMap.get(entity.key) as T | undefined;
      viewMap.set(entity.key, entity.data);
      this.notifyUpdate(viewPath, entity.key, {
        type: 'upsert',
        key: entity.key,
        data: entity.data,
      });
      this.notifyRichUpdate(viewPath, entity.key, previousValue, entity.data, 'upsert');
    }
  }

  private handleEntityFrame<T>(frame: EntityFrame<T>): void {
    const viewPath = frame.entity;
    let viewMap = this.entities.get(viewPath);

    if (!viewMap) {
      viewMap = new Map();
      this.entities.set(viewPath, viewMap);
    }

    const previousValue = viewMap.get(frame.key) as T | undefined;

    switch (frame.op) {
      case 'create':
      case 'upsert':
        viewMap.set(frame.key, frame.data);
        this.notifyUpdate(viewPath, frame.key, {
          type: 'upsert',
          key: frame.key,
          data: frame.data,
        });
        this.notifyRichUpdate(viewPath, frame.key, previousValue, frame.data, frame.op);
        break;

      case 'patch': {
        const existing = viewMap.get(frame.key);
        const merged = existing ? deepMerge(existing, frame.data as Partial<unknown>) : frame.data;
        viewMap.set(frame.key, merged);
        this.notifyUpdate(viewPath, frame.key, {
          type: 'patch',
          key: frame.key,
          data: frame.data as Partial<unknown>,
        });
        this.notifyRichUpdate(viewPath, frame.key, previousValue, merged as T, 'patch', frame.data);
        break;
      }

      case 'delete':
        viewMap.delete(frame.key);
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
    const viewMap = this.entities.get(viewPath);
    if (!viewMap) return [];
    return Array.from(viewMap.values()) as T[];
  }

  get<T>(viewPath: string, key: string): T | null {
    const viewMap = this.entities.get(viewPath);
    if (!viewMap) return null;
    const value = viewMap.get(key);
    return value !== undefined ? (value as T) : null;
  }

  getAllSync<T>(viewPath: string): T[] | undefined {
    const viewMap = this.entities.get(viewPath);
    if (!viewMap) return undefined;
    return Array.from(viewMap.values()) as T[];
  }

  getSync<T>(viewPath: string, key: string): T | null | undefined {
    const viewMap = this.entities.get(viewPath);
    if (!viewMap) return undefined;
    const value = viewMap.get(key);
    return value !== undefined ? (value as T) : null;
  }

  keys(viewPath: string): string[] {
    const viewMap = this.entities.get(viewPath);
    if (!viewMap) return [];
    return Array.from(viewMap.keys());
  }

  size(viewPath: string): number {
    const viewMap = this.entities.get(viewPath);
    return viewMap?.size ?? 0;
  }

  clear(): void {
    this.entities.clear();
  }

  clearView(viewPath: string): void {
    this.entities.delete(viewPath);
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

import type { Frame, SnapshotFrame, EntityFrame, SubscribedFrame } from './frame';
import { isSnapshotFrame, isSubscribedFrame } from './frame';
import type { StorageAdapter } from './storage/adapter';
import type { RichUpdate } from './types';
import { DEFAULT_MAX_ENTRIES_PER_VIEW } from './types';

export interface FrameProcessorConfig {
  maxEntriesPerView?: number | null;
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

export class FrameProcessor {
  private storage: StorageAdapter;
  private maxEntriesPerView: number | null;

  constructor(storage: StorageAdapter, config: FrameProcessorConfig = {}) {
    this.storage = storage;
    this.maxEntriesPerView = config.maxEntriesPerView === undefined
      ? DEFAULT_MAX_ENTRIES_PER_VIEW
      : config.maxEntriesPerView;
  }

  handleFrame<T>(frame: Frame<T>): void {
    if (isSubscribedFrame(frame)) {
      this.handleSubscribedFrame(frame);
    } else if (isSnapshotFrame(frame)) {
      this.handleSnapshotFrame(frame);
    } else {
      this.handleEntityFrame(frame);
    }
  }

  private handleSubscribedFrame(frame: SubscribedFrame): void {
    if (this.storage.setViewConfig && frame.sort) {
      this.storage.setViewConfig(frame.view, { sort: frame.sort });
    }
  }

  private handleSnapshotFrame<T>(frame: SnapshotFrame<T>): void {
    const viewPath = frame.entity;

    for (const entity of frame.data) {
      const previousValue = this.storage.get<T>(viewPath, entity.key);
      this.storage.set(viewPath, entity.key, entity.data);

      this.storage.notifyUpdate(viewPath, entity.key, {
        type: 'upsert',
        key: entity.key,
        data: entity.data,
      });

      this.emitRichUpdate(viewPath, entity.key, previousValue, entity.data, 'upsert');
    }

    this.enforceMaxEntries(viewPath);
  }

  private handleEntityFrame<T>(frame: EntityFrame<T>): void {
    const viewPath = frame.entity;
    const previousValue = this.storage.get<T>(viewPath, frame.key);

    switch (frame.op) {
      case 'create':
      case 'upsert':
        this.storage.set(viewPath, frame.key, frame.data);
        this.enforceMaxEntries(viewPath);
        this.storage.notifyUpdate(viewPath, frame.key, {
          type: 'upsert',
          key: frame.key,
          data: frame.data,
        });
        this.emitRichUpdate(viewPath, frame.key, previousValue, frame.data, frame.op);
        break;

      case 'patch': {
        const existing = this.storage.get<T>(viewPath, frame.key);
        const appendPaths = frame.append ?? [];
        const merged = existing
          ? deepMergeWithAppend(existing, frame.data as Partial<T>, appendPaths)
          : frame.data;
        this.storage.set(viewPath, frame.key, merged);
        this.enforceMaxEntries(viewPath);
        this.storage.notifyUpdate(viewPath, frame.key, {
          type: 'patch',
          key: frame.key,
          data: frame.data as Partial<T>,
        });
        this.emitRichUpdate(viewPath, frame.key, previousValue, merged as T, 'patch', frame.data);
        break;
      }

      case 'delete':
        this.storage.delete(viewPath, frame.key);
        this.storage.notifyUpdate(viewPath, frame.key, {
          type: 'delete',
          key: frame.key,
        });
        if (previousValue !== null) {
          const richUpdate: RichUpdate<T> = { type: 'deleted', key: frame.key, lastKnown: previousValue };
          this.storage.notifyRichUpdate(viewPath, frame.key, richUpdate);
        }
        break;
    }
  }

  private emitRichUpdate<T>(
    viewPath: string,
    key: string,
    before: T | null,
    after: T,
    _op: 'create' | 'upsert' | 'patch',
    patch?: unknown
  ): void {
    const richUpdate: RichUpdate<T> = before === null
      ? { type: 'created', key, data: after }
      : { type: 'updated', key, before, after, patch };

    this.storage.notifyRichUpdate(viewPath, key, richUpdate);
  }

  private enforceMaxEntries(viewPath: string): void {
    if (this.maxEntriesPerView === null) return;
    if (!this.storage.evictOldest) return;

    while (this.storage.size(viewPath) > this.maxEntriesPerView) {
      this.storage.evictOldest(viewPath);
    }
  }
}

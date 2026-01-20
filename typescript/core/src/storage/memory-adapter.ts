import type { StorageAdapter, UpdateCallback, RichUpdateCallback, StorageAdapterConfig } from './adapter';
import type { Update, RichUpdate } from '../types';

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

  clear(): void {
    this.entities.clear();
    this.accessOrder = [];
  }
}

export class MemoryAdapter implements StorageAdapter {
  private views: Map<string, ViewData<unknown>> = new Map();
  private updateCallbacks: Set<UpdateCallback> = new Set();
  private richUpdateCallbacks: Set<RichUpdateCallback> = new Set();

  constructor(_config: StorageAdapterConfig = {}) {}

  get<T>(viewPath: string, key: string): T | null {
    const view = this.views.get(viewPath);
    if (!view) return null;
    const value = view.get(key);
    return value !== undefined ? (value as T) : null;
  }

  getAll<T>(viewPath: string): T[] {
    const view = this.views.get(viewPath);
    if (!view) return [];
    return Array.from(view.values()) as T[];
  }

  getAllSync<T>(viewPath: string): T[] | undefined {
    const view = this.views.get(viewPath);
    if (!view) return undefined;
    return Array.from(view.values()) as T[];
  }

  getSync<T>(viewPath: string, key: string): T | null | undefined {
    const view = this.views.get(viewPath);
    if (!view) return undefined;
    const value = view.get(key);
    return value !== undefined ? (value as T) : null;
  }

  has(viewPath: string, key: string): boolean {
    return this.views.get(viewPath)?.has(key) ?? false;
  }

  keys(viewPath: string): string[] {
    const view = this.views.get(viewPath);
    if (!view) return [];
    return Array.from(view.keys());
  }

  size(viewPath: string): number {
    return this.views.get(viewPath)?.size ?? 0;
  }

  set<T>(viewPath: string, key: string, data: T): void {
    let view = this.views.get(viewPath);
    if (!view) {
      view = new ViewData();
      this.views.set(viewPath, view);
    }
    view.set(key, data);
  }

  delete(viewPath: string, key: string): void {
    this.views.get(viewPath)?.delete(key);
  }

  clear(viewPath?: string): void {
    if (viewPath) {
      this.views.get(viewPath)?.clear();
      this.views.delete(viewPath);
    } else {
      this.views.clear();
    }
  }

  evictOldest(viewPath: string): string | undefined {
    return this.views.get(viewPath)?.evictOldest();
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
}

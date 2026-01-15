import type { Subscription, UnsubscribeFn } from './types';
import type { ConnectionManager } from './connection';

interface SubscriptionTracker {
  subscription: Subscription;
  refCount: number;
}

type SubKey = string;

export class SubscriptionRegistry {
  private subscriptions: Map<SubKey, SubscriptionTracker> = new Map();
  private connection: ConnectionManager;

  constructor(connection: ConnectionManager) {
    this.connection = connection;
  }

  subscribe(subscription: Subscription): UnsubscribeFn {
    const subKey = this.makeSubKey(subscription);
    const existing = this.subscriptions.get(subKey);

    if (existing) {
      existing.refCount++;
    } else {
      this.subscriptions.set(subKey, {
        subscription,
        refCount: 1,
      });
      this.connection.subscribe(subscription);
    }

    return () => this.unsubscribe(subscription);
  }

  unsubscribe(subscription: Subscription): void {
    const subKey = this.makeSubKey(subscription);
    const existing = this.subscriptions.get(subKey);

    if (existing) {
      existing.refCount--;
      if (existing.refCount <= 0) {
        this.subscriptions.delete(subKey);
        this.connection.unsubscribe(subscription.view, subscription.key);
      }
    }
  }

  getRefCount(subscription: Subscription): number {
    const subKey = this.makeSubKey(subscription);
    return this.subscriptions.get(subKey)?.refCount ?? 0;
  }

  getActiveSubscriptions(): Subscription[] {
    return Array.from(this.subscriptions.values()).map((t) => t.subscription);
  }

  clear(): void {
    for (const { subscription } of this.subscriptions.values()) {
      this.connection.unsubscribe(subscription.view, subscription.key);
    }
    this.subscriptions.clear();
  }

  private makeSubKey(subscription: Subscription): SubKey {
    const filters = subscription.filters ? JSON.stringify(subscription.filters) : '{}';
    return `${subscription.view}:${subscription.key ?? '*'}:${subscription.partition ?? ''}:${filters}`;
  }
}

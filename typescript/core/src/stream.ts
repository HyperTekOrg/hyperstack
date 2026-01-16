import type { Update, RichUpdate, Subscription, UnsubscribeFn } from './types';
import type { EntityStore } from './store';
import type { SubscriptionRegistry } from './subscription';

const MAX_QUEUE_SIZE = 1000;

type UpdateQueueItem<T> = {
  update: Update<T>;
  resolve: () => void;
};

type RichUpdateQueueItem<T> = {
  update: RichUpdate<T>;
  resolve: () => void;
};

export function createUpdateStream<T>(
  store: EntityStore,
  subscriptionRegistry: SubscriptionRegistry,
  subscription: Subscription,
  keyFilter?: string
): AsyncIterable<Update<T>> {
  return {
    [Symbol.asyncIterator](): AsyncIterator<Update<T>> {
      const queue: UpdateQueueItem<T>[] = [];
      let waitingResolve: ((value: IteratorResult<Update<T>>) => void) | null = null;
      let unsubscribeStore: UnsubscribeFn | null = null;
      let unsubscribeRegistry: UnsubscribeFn | null = null;
      let done = false;

      const handler = (viewPath: string, key: string, update: Update<unknown>) => {
        if (viewPath !== subscription.view) return;
        if (keyFilter !== undefined && key !== keyFilter) return;

        const typedUpdate = update as Update<T>;

        if (waitingResolve) {
          const resolve = waitingResolve;
          waitingResolve = null;
          resolve({ value: typedUpdate, done: false });
        } else {
          if (queue.length >= MAX_QUEUE_SIZE) {
            queue.shift();
          }
          queue.push({
            update: typedUpdate,
            resolve: () => {},
          });
        }
      };

      const start = () => {
        unsubscribeStore = store.onUpdate(handler);
        unsubscribeRegistry = subscriptionRegistry.subscribe(subscription);
      };

      const cleanup = () => {
        done = true;
        unsubscribeStore?.();
        unsubscribeRegistry?.();
      };

      start();

      return {
        async next(): Promise<IteratorResult<Update<T>>> {
          if (done) {
            return { value: undefined, done: true };
          }

          const queued = queue.shift();
          if (queued) {
            return { value: queued.update, done: false };
          }

          return new Promise((resolve) => {
            waitingResolve = resolve;
          });
        },

        async return(): Promise<IteratorResult<Update<T>>> {
          cleanup();
          return { value: undefined, done: true };
        },

        async throw(error?: unknown): Promise<IteratorResult<Update<T>>> {
          cleanup();
          throw error;
        },
      };
    },
  };
}

export function createRichUpdateStream<T>(
  store: EntityStore,
  subscriptionRegistry: SubscriptionRegistry,
  subscription: Subscription,
  keyFilter?: string
): AsyncIterable<RichUpdate<T>> {
  return {
    [Symbol.asyncIterator](): AsyncIterator<RichUpdate<T>> {
      const queue: RichUpdateQueueItem<T>[] = [];
      let waitingResolve: ((value: IteratorResult<RichUpdate<T>>) => void) | null = null;
      let unsubscribeStore: UnsubscribeFn | null = null;
      let unsubscribeRegistry: UnsubscribeFn | null = null;
      let done = false;

      const handler = (viewPath: string, key: string, update: RichUpdate<unknown>) => {
        if (viewPath !== subscription.view) return;
        if (keyFilter !== undefined && key !== keyFilter) return;

        const typedUpdate = update as RichUpdate<T>;

        if (waitingResolve) {
          const resolve = waitingResolve;
          waitingResolve = null;
          resolve({ value: typedUpdate, done: false });
        } else {
          if (queue.length >= MAX_QUEUE_SIZE) {
            queue.shift();
          }
          queue.push({
            update: typedUpdate,
            resolve: () => {},
          });
        }
      };

      const start = () => {
        unsubscribeStore = store.onRichUpdate(handler);
        unsubscribeRegistry = subscriptionRegistry.subscribe(subscription);
      };

      const cleanup = () => {
        done = true;
        unsubscribeStore?.();
        unsubscribeRegistry?.();
      };

      start();

      return {
        async next(): Promise<IteratorResult<RichUpdate<T>>> {
          if (done) {
            return { value: undefined, done: true };
          }

          const queued = queue.shift();
          if (queued) {
            return { value: queued.update, done: false };
          }

          return new Promise((resolve) => {
            waitingResolve = resolve;
          });
        },

        async return(): Promise<IteratorResult<RichUpdate<T>>> {
          cleanup();
          return { value: undefined, done: true };
        },

        async throw(error?: unknown): Promise<IteratorResult<RichUpdate<T>>> {
          cleanup();
          throw error;
        },
      };
    },
  };
}

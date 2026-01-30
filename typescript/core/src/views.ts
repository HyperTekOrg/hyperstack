import type {
  Update,
  RichUpdate,
  TypedStateView,
  TypedListView,
  ViewDef,
  StackDefinition,
  TypedViews,
  WatchOptions,
} from './types';
import type { StorageAdapter } from './storage/adapter';
import type { SubscriptionRegistry } from './subscription';
import { createUpdateStream, createEntityStream, createRichUpdateStream } from './stream';

export function createTypedStateView<T>(
  viewDef: ViewDef<T, 'state'>,
  storage: StorageAdapter,
  subscriptionRegistry: SubscriptionRegistry
): TypedStateView<T> {
  return {
    use(key: string, options?: WatchOptions): AsyncIterable<T> {
      return createEntityStream<T>(
        storage,
        subscriptionRegistry,
        { view: viewDef.view, key, ...options },
        key
      );
    },

    watch(key: string, options?: WatchOptions): AsyncIterable<Update<T>> {
      return createUpdateStream<T>(
        storage,
        subscriptionRegistry,
        { view: viewDef.view, key, ...options },
        key
      );
    },

    watchRich(key: string, options?: WatchOptions): AsyncIterable<RichUpdate<T>> {
      return createRichUpdateStream<T>(
        storage,
        subscriptionRegistry,
        { view: viewDef.view, key, ...options },
        key
      );
    },

    async get(key: string): Promise<T | null> {
      return storage.get<T>(viewDef.view, key);
    },

    getSync(key: string): T | null | undefined {
      return storage.getSync<T>(viewDef.view, key);
    },
  };
}

export function createTypedListView<T>(
  viewDef: ViewDef<T, 'list'>,
  storage: StorageAdapter,
  subscriptionRegistry: SubscriptionRegistry
): TypedListView<T> {
  return {
    use(options?: WatchOptions): AsyncIterable<T> {
      return createEntityStream<T>(storage, subscriptionRegistry, { view: viewDef.view, ...options });
    },

    watch(options?: WatchOptions): AsyncIterable<Update<T>> {
      return createUpdateStream<T>(storage, subscriptionRegistry, { view: viewDef.view, ...options });
    },

    watchRich(options?: WatchOptions): AsyncIterable<RichUpdate<T>> {
      return createRichUpdateStream<T>(storage, subscriptionRegistry, { view: viewDef.view, ...options });
    },

    async get(): Promise<T[]> {
      return storage.getAll<T>(viewDef.view);
    },

    getSync(): T[] | undefined {
      return storage.getAllSync<T>(viewDef.view);
    },
  };
}

export function createTypedViews<TStack extends StackDefinition>(
  stack: TStack,
  storage: StorageAdapter,
  subscriptionRegistry: SubscriptionRegistry
): TypedViews<TStack['views']> {
  const views = {} as Record<string, Record<string, unknown>>;

  for (const [entityName, viewGroup] of Object.entries(stack.views)) {
    const group = viewGroup as Record<string, ViewDef<unknown, 'state' | 'list'>>;
    const typedGroup: Record<string, unknown> = {};

    for (const [viewName, viewDef] of Object.entries(group)) {
      if (viewDef.mode === 'state') {
        typedGroup[viewName] = createTypedStateView(viewDef as ViewDef<unknown, 'state'>, storage, subscriptionRegistry);
      } else if (viewDef.mode === 'list') {
        typedGroup[viewName] = createTypedListView(viewDef as ViewDef<unknown, 'list'>, storage, subscriptionRegistry);
      }
    }

    views[entityName] = typedGroup;
  }

  return views as TypedViews<TStack['views']>;
}

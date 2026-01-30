import { useEffect, useState, useCallback, useSyncExternalStore, useRef, useMemo } from 'react';
import { ViewDef, ViewHookOptions, ViewHookResult, ListParams, ListParamsBase } from './types';
import type { HyperStack } from 'hyperstack-typescript';

export function createStateViewHook<T>(
  viewDef: ViewDef<T, 'state'>,
  client: HyperStack<any>
) {
  return {
    use: (key?: Record<string, string>, options?: ViewHookOptions): ViewHookResult<T> => {
      const [isLoading, setIsLoading] = useState(!options?.initialData);
      const [error, setError] = useState<Error | undefined>();

      const keyString = key ? Object.values(key)[0] : undefined;
      const enabled = options?.enabled !== false;

      useEffect(() => {
        if (!enabled) return undefined;

        try {
          const registry = client.getSubscriptionRegistry();
          const unsubscribe = registry.subscribe({ view: viewDef.view, key: keyString });
          setIsLoading(true);

          return () => {
            try {
              unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error unsubscribing from view:', err);
            }
          };
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Subscription failed'));
          setIsLoading(false);
          return undefined;
        }
      }, [keyString, enabled, client]);

      const refresh = useCallback(() => {
        if (!enabled) return;

        try {
          const registry = client.getSubscriptionRegistry();
          const unsubscribe = registry.subscribe({ view: viewDef.view, key: keyString });
          setIsLoading(true);

          setTimeout(() => {
            try {
              unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error during refresh unsubscribe:', err);
            }
          }, 0);
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Refresh failed'));
          setIsLoading(false);
        }
      }, [keyString, enabled, client]);

      const store = client.store;
      const data = useSyncExternalStore(
        (callback) => store.onUpdate(callback),
        () => {
          const entity = keyString 
            ? store.get(viewDef.view, keyString)
            : store.getAll(viewDef.view)[0];
          return entity as T | undefined;
        }
      );

      useEffect(() => {
        if (data && isLoading) {
          setIsLoading(false);
        }
      }, [data, isLoading]);

      return {
        data: (options?.initialData ?? data) as T | undefined,
        isLoading,
        error,
        refresh
      };
    }
  };
}

function useListViewInternal<T>(
  viewDef: ViewDef<T, 'list'>,
  client: HyperStack<any>,
  params?: ListParams,
  options?: ViewHookOptions
): ViewHookResult<T[]> {
  const [isLoading, setIsLoading] = useState(!options?.initialData);
  const [error, setError] = useState<Error | undefined>();
  const cachedDataRef = useRef<T[] | undefined>(undefined);

  const enabled = options?.enabled !== false;
  const key = params?.key;

  const filtersJson = params?.filters ? JSON.stringify(params.filters) : undefined;
  const filters = useMemo(() => params?.filters, [filtersJson]);

  useEffect(() => {
    if (!enabled) return undefined;

    try {
      const registry = client.getSubscriptionRegistry();
      const unsubscribe = registry.subscribe({ 
        view: viewDef.view, 
        key, 
        filters,
        take: params?.take,
        skip: params?.skip 
      });
      setIsLoading(true);

      return () => {
        try {
          unsubscribe();
        } catch (err) {
          console.error('[Hyperstack] Error unsubscribing from list view:', err);
        }
      };
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Subscription failed'));
      setIsLoading(false);
      return undefined;
    }
  }, [enabled, key, filtersJson, params?.take, params?.skip, client]);

  const refresh = useCallback(() => {
    if (!enabled) return;

    try {
      const registry = client.getSubscriptionRegistry();
      const unsubscribe = registry.subscribe({ 
        view: viewDef.view, 
        key, 
        filters,
        take: params?.take,
        skip: params?.skip 
      });
      setIsLoading(true);

      setTimeout(() => {
        try {
          unsubscribe();
        } catch (err) {
          console.error('[Hyperstack] Error during list refresh unsubscribe:', err);
        }
      }, 0);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Refresh failed'));
      setIsLoading(false);
    }
  }, [enabled, key, filtersJson, params?.take, params?.skip, client]);

  const store = client.store;
  const data = useSyncExternalStore(
    (callback) => store.onUpdate(callback),
    () => {
      const viewData = store.getAll(viewDef.view);
      
      if (!viewData || viewData.length === 0) {
        cachedDataRef.current = undefined;
        return undefined;
      }

      let items = viewData;
      
      if (params?.where) {
        items = items.filter((item) => {
          return Object.entries(params.where!).every(([fieldKey, condition]) => {
            const value = (item as Record<string, unknown>)[fieldKey];

            if (typeof condition === 'object' && condition !== null) {
              const cond = condition as Record<string, unknown>;
              if ('gte' in cond) return (value as number) >= (cond.gte as number);
              if ('lte' in cond) return (value as number) <= (cond.lte as number);
              if ('gt' in cond) return (value as number) > (cond.gt as number);
              if ('lt' in cond) return (value as number) < (cond.lt as number);
            }

            return value === condition;
          });
        });
      }

      if (params?.limit) {
        items = items.slice(0, params.limit);
      }

      cachedDataRef.current = items as T[];
      return items as T[];
    }
  );

  useEffect(() => {
    if (data && isLoading) {
      setIsLoading(false);
    }
  }, [data, isLoading]);

  return {
    data: (options?.initialData ?? data) as T[] | undefined,
    isLoading,
    error,
    refresh
  };
}

export function createListViewHook<T>(
  viewDef: ViewDef<T, 'list'>,
  client: HyperStack<any>
) {
  function use(params?: ListParams, options?: ViewHookOptions): ViewHookResult<T[]> | ViewHookResult<T | undefined> {
    const result = useListViewInternal(viewDef, client, params, options);
    
    if (params?.take === 1) {
      return {
        data: result.data?.[0],
        isLoading: result.isLoading,
        error: result.error,
        refresh: result.refresh
      } as ViewHookResult<T | undefined>;
    }
    
    return result;
  }

  function useOne(params?: Omit<ListParamsBase, 'take'>, options?: ViewHookOptions): ViewHookResult<T | undefined> {
    const paramsWithTake = params ? { ...params, take: 1 as const } : { take: 1 as const };
    const result = useListViewInternal(viewDef, client, paramsWithTake, options);
    
    return {
      data: result.data?.[0],
      isLoading: result.isLoading,
      error: result.error,
      refresh: result.refresh
    };
  }

  return { use, useOne };
}

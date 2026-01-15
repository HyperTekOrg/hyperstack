import { useEffect, useState, useCallback, useSyncExternalStore, useRef, useMemo } from 'react';
import { ViewDefinition, ViewHookOptions, ViewHookResult, ListParams } from './types';
import { HyperstackRuntime } from './runtime';

export function createStateViewHook<T>(
  viewDef: ViewDefinition<T>,
  runtime: HyperstackRuntime
) {
  return {
    use: (key: Record<string, string>, options?: ViewHookOptions): ViewHookResult<T> => {
      const [isLoading, setIsLoading] = useState(!options?.initialData);
      const [error, setError] = useState<Error | undefined>();

      const keyString = Object.values(key)[0];
      const enabled = options?.enabled !== false;

      useEffect(() => {
        if (!enabled) return undefined;

        try {
          const handle = runtime.subscribe(viewDef.view, keyString);
          setIsLoading(true);

          return () => {
            try {
              handle.unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error unsubscribing from view:', err);
            }
          };
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Subscription failed'));
          setIsLoading(false);
          return undefined;
        }
      }, [keyString, enabled]);

      const refresh = useCallback(() => {
        if (!enabled) return;

        try {
          const handle = runtime.subscribe(viewDef.view, keyString);
          setIsLoading(true);

          setTimeout(() => {
            try {
              handle.unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error during refresh unsubscribe:', err);
            }
          }, 0);
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Refresh failed'));
          setIsLoading(false);
        }
      }, [keyString, enabled]);

      const data = useSyncExternalStore(
        (callback) => {
          const unsubscribe = runtime.store.subscribe(() => {
            callback();
          });
          return unsubscribe;
        },
        () => {
          const rawData = runtime.store.getState().entities.get(viewDef.view)?.get(keyString);

          if (rawData && viewDef.transform) {
            try {
              return viewDef.transform(rawData) as T;
            } catch (err) {
              return undefined;
            }
          }

          return rawData as T | undefined;
        }
      );

      useEffect(() => {
        if (data && isLoading) {
          setIsLoading(false);
        }
      }, [data, isLoading]);

      return {
        data: options?.initialData ?? data,
        isLoading,
        error,
        refresh
      };
    }
  };
}

export function createListViewHook<T>(
  viewDef: ViewDefinition<T>,
  runtime: HyperstackRuntime
) {
  return {
    use: (params?: ListParams, options?: ViewHookOptions): ViewHookResult<T[]> => {
      const [isLoading, setIsLoading] = useState(!options?.initialData);
      const [error, setError] = useState<Error | undefined>();
      const cachedDataRef = useRef<T[] | undefined>(undefined);
      const lastMapRef = useRef<Map<string, any> | undefined>(undefined);

      const enabled = options?.enabled !== false;
      const key = params?.key;

      // Stabilize filters object to prevent unnecessary re-subscriptions
      // We use a JSON string as the dependency to detect actual value changes
      const filtersJson = params?.filters ? JSON.stringify(params.filters) : undefined;
      const filters = useMemo(() => params?.filters, [filtersJson]);

      useEffect(() => {
        if (!enabled) return undefined;

        try {
          const handle = runtime.subscribe(viewDef.view, key, filters);
          setIsLoading(true);

          return () => {
            try {
              handle.unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error unsubscribing from list view:', err);
            }
          };
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Subscription failed'));
          setIsLoading(false);
          return undefined;
        }
      }, [enabled, key, filtersJson]);

      const refresh = useCallback(() => {
        if (!enabled) return;

        try {
          const handle = runtime.subscribe(viewDef.view, key, filters);
          setIsLoading(true);

          setTimeout(() => {
            try {
              handle.unsubscribe();
            } catch (err) {
              console.error('[Hyperstack] Error during list refresh unsubscribe:', err);
            }
          }, 0);
        } catch (err) {
          setError(err instanceof Error ? err : new Error('Refresh failed'));
          setIsLoading(false);
        }
      }, [enabled, key, filtersJson]);

      const data = useSyncExternalStore(
        (callback) => {
          const unsubscribe = runtime.store.subscribe(() => {
            callback();
          });
          return unsubscribe;
        },
        () => {
          let baseMap = runtime.store.getState().entities.get(viewDef.view) as Map<string, any> | undefined;

          if (!baseMap) {
            if (cachedDataRef.current !== undefined) {
              cachedDataRef.current = undefined;
              lastMapRef.current = undefined;
            }
            return undefined;
          }

          if (lastMapRef.current === baseMap && cachedDataRef.current !== undefined) {
            return cachedDataRef.current;
          }

          let items = Array.from(baseMap.values()).map((value: any) => {
            if (viewDef.transform) {
              try {
                return viewDef.transform(value) as T;
              } catch (err) {
                console.log("Error transforming", err);
                return value;
              }
            }

            return value;
          });

          if (params?.where) {
            items = items.filter((item) => {
              return Object.entries(params.where!).every(([key, condition]) => {
                const value = (item as any)[key];

                if (typeof condition === 'object' && condition !== null) {
                  if ('gte' in condition) return value >= condition.gte;
                  if ('lte' in condition) return value <= condition.lte;
                  if ('gt' in condition) return value > condition.gt;
                  if ('lt' in condition) return value < condition.lt;
                }

                return value === condition;
              });
            });
          }

          if (params?.limit) {
            items = items.slice(0, params.limit);
          }

          lastMapRef.current = runtime.store.getState().entities.get(viewDef.view) as Map<string, any> | undefined;
          cachedDataRef.current = items;
          return items;
        }
      );

      useEffect(() => {
        if (data && isLoading) {
          setIsLoading(false);
        }
      }, [data, isLoading]);

      return {
        data: options?.initialData ?? data,
        isLoading,
        error,
        refresh
      };
    }
  };
}

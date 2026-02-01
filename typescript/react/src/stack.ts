import { useEffect, useState, useMemo } from 'react';
import type { StoreApi, UseBoundStore } from 'zustand';
import { useHyperstackContext } from './provider';
import { createStateViewHook, createListViewHook } from './view-hooks';
import { useInstructionMutation, UseMutationResult } from './hooks';
import type {
  StackDefinition,
  ViewDef,
  ViewMode,
  ViewHookOptions,
  ViewHookResult,
  ListParamsSingle,
  ListParamsMultiple,
  ListParamsBase,
  ViewGroup,
  UseHyperstackOptions
} from './types';
import { ZustandAdapter, type HyperStackStore } from './zustand-adapter';
import type { InstructionHandler, InstructionExecutor } from 'hyperstack-typescript';
import type { HyperStack } from 'hyperstack-typescript';

type ViewHookForDef<TDef> = TDef extends ViewDef<infer T, 'state'>
  ? {
      use: (
        key?: Record<string, string>,
        options?: ViewHookOptions
      ) => ViewHookResult<T>;
    }
  : TDef extends ViewDef<infer T, 'list'>
  ? {
      use: {
        (params: ListParamsSingle, options?: ViewHookOptions): ViewHookResult<T | undefined>;
        (params?: ListParamsMultiple, options?: ViewHookOptions): ViewHookResult<T[]>;
      };
      useOne: (
        params?: Omit<ListParamsBase, 'take'>,
        options?: ViewHookOptions
      ) => ViewHookResult<T | undefined>;
    }
  : TDef extends ViewDef<infer T, 'state' | 'list'>
  ? {
      use: {
        (params: ListParamsSingle, options?: ViewHookOptions): ViewHookResult<T | undefined>;
        (params?: ListParamsMultiple | Record<string, string>, options?: ViewHookOptions): ViewHookResult<T | T[]>;
      };
      useOne: (
        params?: Omit<ListParamsBase, 'take'>,
        options?: ViewHookOptions
      ) => ViewHookResult<T | undefined>;
    }
  : never;

type BuildViewInterface<TViews extends Record<string, ViewGroup>> = {
  [K in keyof TViews]: {
    [SubK in keyof TViews[K] as TViews[K][SubK] extends ViewDef<unknown, ViewMode> ? SubK : never]: ViewHookForDef<TViews[K][SubK]>;
  };
};

type InstructionHook = {
  useMutation: () => UseMutationResult;
  execute: InstructionExecutor;
};

type BuildInstructionInterface<TInstructions extends Record<string, InstructionHandler> | undefined> = 
  TInstructions extends Record<string, InstructionHandler>
    ? { [K in keyof TInstructions]: InstructionHook }
    : {};

type StackClient<TStack extends StackDefinition> = {
  views: BuildViewInterface<TStack['views']>;
  instructions: BuildInstructionInterface<TStack['instructions']>;
  zustandStore: UseBoundStore<StoreApi<HyperStackStore>>;
  client: HyperStack<TStack>;
  isLoading: boolean;
  error: Error | null;
};

export function useHyperstack<TStack extends StackDefinition>(
  stack: TStack,
  options?: UseHyperstackOptions
): StackClient<TStack> {
  const { getOrCreateClient, getClient } = useHyperstackContext();
  const urlOverride = options?.url;
  const [client, setClient] = useState<HyperStack<TStack> | null>(getClient(stack) as HyperStack<TStack> | null);
  const [isLoading, setIsLoading] = useState(!client);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const existingClient = getClient(stack);
    if (existingClient && !urlOverride) {
      setClient(existingClient as HyperStack<TStack>);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    getOrCreateClient(stack, urlOverride)
      .then((newClient) => {
        setClient(newClient as HyperStack<TStack>);
        setIsLoading(false);
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      });
  }, [stack, getOrCreateClient, getClient, urlOverride]);

  const views = useMemo(() => {
    const result: Record<string, Record<string, unknown>> = {};

    for (const [viewName, viewGroup] of Object.entries(stack.views)) {
      result[viewName] = {};

      if (typeof viewGroup === 'object' && viewGroup !== null) {
        for (const [subViewName, viewDef] of Object.entries(viewGroup)) {
          if (!viewDef || typeof viewDef !== 'object' || !('mode' in viewDef)) continue;

          if (viewDef.mode === 'state') {
            result[viewName]![subViewName] = createStateViewHook(viewDef as ViewDef<unknown, 'state'>, client);
          } else if (viewDef.mode === 'list') {
            result[viewName]![subViewName] = createListViewHook(viewDef as ViewDef<unknown, 'list'>, client);
          }
        }
      }
    }

    return result;
  }, [stack, client]);

  const instructions = useMemo(() => {
    const result: Record<string, InstructionHook> = {};

    if (client?.instructions) {
      for (const [instructionName, executeFn] of Object.entries(client.instructions)) {
        result[instructionName] = {
          execute: executeFn as InstructionExecutor,
          useMutation: () => useInstructionMutation(executeFn as InstructionExecutor)
        };
      }
    }

    return result;
  }, [client]);

  return {
    views: views as BuildViewInterface<TStack['views']>,
    instructions: instructions as BuildInstructionInterface<TStack['instructions']>,
    zustandStore: (client?.store as ZustandAdapter | undefined)?.store as UseBoundStore<StoreApi<HyperStackStore>>,
    client: client!,
    isLoading,
    error
  };
}

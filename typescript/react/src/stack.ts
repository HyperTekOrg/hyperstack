import { useEffect, useState, useCallback } from 'react';
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
  ViewGroup
} from './types';
import type { HyperStackStore } from './zustand-adapter';
import type { InstructionDefinition, InstructionExecutor } from 'hyperstack-typescript';
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

type BuildInstructionInterface<TInstructions extends Record<string, InstructionDefinition> | undefined> = 
  TInstructions extends Record<string, InstructionDefinition>
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
  stack: TStack
): StackClient<TStack> {
  const { getOrCreateClient, getClient } = useHyperstackContext();
  const [client, setClient] = useState<HyperStack<TStack> | null>(getClient(stack) as HyperStack<TStack> | null);
  const [isLoading, setIsLoading] = useState(!client);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const existingClient = getClient(stack);
    if (existingClient) {
      setClient(existingClient as HyperStack<TStack>);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    getOrCreateClient(stack)
      .then((newClient) => {
        setClient(newClient as HyperStack<TStack>);
        setIsLoading(false);
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      });
  }, [stack, getOrCreateClient, getClient]);

  const views: Record<string, Record<string, unknown>> = {};

  if (client) {
    for (const [viewName, viewGroup] of Object.entries(client.views)) {
      views[viewName] = {};

      if (typeof viewGroup === 'object' && viewGroup !== null) {
        for (const [subViewName, viewDef] of Object.entries(viewGroup)) {
          if (!viewDef || typeof viewDef !== 'object' || !('mode' in viewDef)) continue;

          if (viewDef.mode === 'state') {
            views[viewName]![subViewName] = createStateViewHook(viewDef as ViewDef<unknown, 'state'>, client);
          } else if (viewDef.mode === 'list') {
            views[viewName]![subViewName] = createListViewHook(viewDef as ViewDef<unknown, 'list'>, client);
          }
        }
      }
    }
  }

  const instructions: Record<string, InstructionHook> = {};

  if (client?.instructions) {
    for (const [instructionName, executeFn] of Object.entries(client.instructions)) {
      instructions[instructionName] = {
        execute: executeFn as InstructionExecutor,
        useMutation: () => useInstructionMutation(executeFn as InstructionExecutor)
      };
    }
  }

  return {
    views: views as BuildViewInterface<TStack['views']>,
    instructions: instructions as BuildInstructionInterface<TStack['instructions']>,
    zustandStore: client?.store as unknown as UseBoundStore<StoreApi<HyperStackStore>>,
    client: client!,
    isLoading,
    error
  };
}
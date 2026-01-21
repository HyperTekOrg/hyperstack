import type { StoreApi, UseBoundStore } from 'zustand';
import { useHyperstackContext } from './provider';
import { createStateViewHook, createListViewHook } from './view-hooks';
import { createTxMutationHook } from './tx-hooks';
import {
  StackDefinition,
  ViewDef,
  ViewMode,
  TransactionDefinition,
  ViewHookOptions,
  ViewHookResult,
  ListParams,
  UseMutationReturn,
  ViewGroup
} from './types';
import { HyperstackRuntime } from './runtime';
import type { HyperStackStore } from './zustand-adapter';

type ViewHookForDef<TDef> = TDef extends ViewDef<infer T, 'state'>
  ? {
      use: (
        key?: Record<string, string>,
        options?: ViewHookOptions
      ) => ViewHookResult<T>;
    }
  : TDef extends ViewDef<infer T, 'list'>
  ? {
      use: (
        params?: ListParams,
        options?: ViewHookOptions
      ) => ViewHookResult<T[]>;
    }
  : TDef extends ViewDef<infer T, 'state' | 'list'>
  ? {
      use: (
        keyOrParams?: Record<string, string> | ListParams,
        options?: ViewHookOptions
      ) => ViewHookResult<T | T[]>;
    }
  : never;

type BuildViewInterface<TViews extends Record<string, ViewGroup>> = {
  [K in keyof TViews]: {
    [SubK in keyof TViews[K] as TViews[K][SubK] extends ViewDef<unknown, ViewMode> ? SubK : never]: ViewHookForDef<TViews[K][SubK]>;
  };
};

type StackClient<TStack extends StackDefinition> = {
  views: BuildViewInterface<TStack['views']>;
  tx: TStack['transactions'] extends Record<string, TransactionDefinition>
    ? {
        [K in keyof TStack['transactions']]: TStack['transactions'][K]['build'];
      } & {
        useMutation: () => UseMutationReturn;
      }
    : { useMutation: () => UseMutationReturn };
  zustandStore: UseBoundStore<StoreApi<HyperStackStore>>;
  runtime: HyperstackRuntime;
};

export function useHyperstack<TStack extends StackDefinition>(
  stack: TStack
): StackClient<TStack> {
  if (!stack) {
    throw new Error('[Hyperstack] Stack definition is required');
  }

  const { runtime } = useHyperstackContext();

  const views: Record<string, Record<string, unknown>> = {};

  for (const [viewName, viewGroup] of Object.entries(stack.views)) {
    views[viewName] = {};

    if (typeof viewGroup === 'object' && viewGroup !== null) {
      const group = viewGroup as ViewGroup;

      for (const [subViewName, viewDef] of Object.entries(group)) {
        if (!viewDef || typeof viewDef !== 'object' || !('mode' in viewDef)) continue;

        if (viewDef.mode === 'state') {
          views[viewName]![subViewName] = createStateViewHook(viewDef as ViewDef<unknown, 'state'>, runtime);
        } else if (viewDef.mode === 'list') {
          views[viewName]![subViewName] = createListViewHook(viewDef as ViewDef<unknown, 'list'>, runtime);
        }
      }
    }
  }

  const tx: Record<string, unknown> = {};

  if (stack.transactions) {
    for (const [txName, txDef] of Object.entries(stack.transactions)) {
      tx[txName] = txDef.build;
    }
  }

  tx.useMutation = createTxMutationHook(runtime, stack.transactions);

  return {
    views,
    tx,
    zustandStore: runtime.zustandStore,
    runtime
  } as StackClient<TStack>;
}

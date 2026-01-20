import type { StoreApi, UseBoundStore } from 'zustand';
import { useHyperstackContext } from './provider';
import { createStateViewHook, createListViewHook } from './view-hooks';
import { createTxMutationHook } from './tx-hooks';
import {
  StackDefinition,
  ViewDef,
  TransactionDefinition,
  ViewHookOptions,
  ViewHookResult,
  ListParams,
  UseMutationReturn,
  ViewGroup
} from './types';
import { HyperstackRuntime } from './runtime';
import type { HyperStackStore } from './zustand-adapter';

type BuildViewInterface<TViews extends Record<string, ViewGroup>> = {
  [K in keyof TViews]: TViews[K] extends { state: ViewDef<infer S, 'state'>; list: ViewDef<infer L, 'list'> }
    ? {
        state: {
          use: (
            key: Record<string, string>,
            options?: ViewHookOptions
          ) => ViewHookResult<S>;
        };
        list: {
          use: (
            params?: ListParams,
            options?: ViewHookOptions
          ) => ViewHookResult<L[]>;
        };
      }
    : TViews[K] extends { state: ViewDef<infer S, 'state'> }
    ? {
        state: {
          use: (
            key: Record<string, string>,
            options?: ViewHookOptions
          ) => ViewHookResult<S>;
        };
      }
    : TViews[K] extends { list: ViewDef<infer L, 'list'> }
    ? {
        list: {
          use: (
            params?: ListParams,
            options?: ViewHookOptions
          ) => ViewHookResult<L[]>;
        };
      }
    : object;
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

      if (group.state) {
        views[viewName]!.state = createStateViewHook(group.state, runtime);
      }

      if (group.list) {
        views[viewName]!.list = createListViewHook(group.list, runtime);
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

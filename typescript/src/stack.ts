import { useHyperstackContext } from './provider';
import { createStateViewHook, createListViewHook } from './view-hooks';
import { createTxMutationHook } from './tx-hooks';
import {
  StackDefinition,
  ViewDefinition,
  TransactionDefinition,
  ViewHookOptions,
  ViewHookResult,
  ListParams,
  UseMutationReturn
} from './types';
import { HyperstackRuntime } from './runtime';

export function defineStack<
  TViews extends Record<string, any>,
  TTxs extends Record<string, TransactionDefinition>,
  THelpers extends Record<string, (...args: any[]) => any>
>(definition: {
  name: string;
  views: TViews;
  transactions?: TTxs;
  helpers?: THelpers;
}): StackDefinition & { views: TViews; transactions?: TTxs; helpers?: THelpers } {
  if (!definition.name) {
    throw new Error('[Hyperstack] Stack definition must have a name');
  }
  if (!definition.views || typeof definition.views !== 'object') {
    throw new Error('[Hyperstack] Stack definition must have views');
  }
  return definition as any;
}

type InferViewType<T> = T extends ViewDefinition<infer U> ? U : never;

type BuildViewInterface<TViews extends Record<string, any>> = {
  [K in keyof TViews]: TViews[K] extends { state: ViewDefinition<any>; list: ViewDefinition<any> }
    ? {
        state: {
          use: (
            key: Record<string, string>,
            options?: ViewHookOptions
          ) => ViewHookResult<InferViewType<TViews[K]['state']>>;
        };
        list: {
          use: (
            params?: ListParams,
            options?: ViewHookOptions
          ) => ViewHookResult<InferViewType<TViews[K]['list']>[]>;
        };
      }
    : TViews[K] extends { state: ViewDefinition<any> }
    ? {
        state: {
          use: (
            key: Record<string, string>,
            options?: ViewHookOptions
          ) => ViewHookResult<InferViewType<TViews[K]['state']>>;
        };
      }
    : TViews[K] extends { list: ViewDefinition<any> }
    ? {
        list: {
          use: (
            params?: ListParams,
            options?: ViewHookOptions
          ) => ViewHookResult<InferViewType<TViews[K]['list']>[]>;
        };
      }
    : never;
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
  helpers: TStack['helpers'] extends Record<string, (...args: any[]) => any>
    ? TStack['helpers']
    : {};
  store: HyperstackRuntime['store'];
  runtime: HyperstackRuntime;
};

export function useHyperstack<TStack extends StackDefinition>(
  stack: TStack
): StackClient<TStack> {
  if (!stack) {
    throw new Error('[Hyperstack] Stack definition is required');
  }

  const { runtime } = useHyperstackContext();

  const views = {} as any;
  
  try {
    for (const [viewName, viewGroup] of Object.entries(stack.views)) {
      views[viewName] = {};
      
      if (typeof viewGroup === 'object' && viewGroup !== null) {
        if ('state' in viewGroup && viewGroup.state) {
          views[viewName].state = createStateViewHook(viewGroup.state, runtime);
        }
        
        if ('list' in viewGroup && viewGroup.list) {
          views[viewName].list = createListViewHook(viewGroup.list, runtime);
        }
      }
    }
  } catch (err) {
    console.error('[Hyperstack] Error creating view hooks:', err);
    throw err;
  }

  const tx = {} as any;
  try {
    if (stack.transactions) {
      for (const [txName, txDef] of Object.entries(stack.transactions)) {
        tx[txName] = txDef.build;
      }
    }

    tx.useMutation = createTxMutationHook(runtime, stack.transactions);
  } catch (err) {
    console.error('[Hyperstack] Error creating transaction hooks:', err);
    tx.useMutation = () => ({
      submit: async () => {},
      status: 'idle' as const,
      error: 'Failed to initialize transaction hooks',
      signature: undefined,
      reset: () => {}
    });
  }

  return {
    views,
    tx,
    helpers: stack.helpers || {},
    store: runtime.store,
    runtime
  } as StackClient<TStack>;
}

import { ViewDefinition } from './types';

export interface ViewFactoryOptions<T> {
  transform?: (data: any) => T;
}

export function createStateView<T>(
  viewPath: string,
  options?: ViewFactoryOptions<T>
): ViewDefinition<T> {
  return {
    mode: 'state' as const,
    view: viewPath,
    type: {} as T,
    transform: options?.transform
  };
}

export function createListView<T>(
  viewPath: string,
  options?: ViewFactoryOptions<T>
): ViewDefinition<T> {
  return {
    mode: 'list' as const,
    view: viewPath,
    type: {} as T,
    transform: options?.transform
  };
}

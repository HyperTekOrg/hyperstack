export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'error';

export type Update<T> =
  | { type: 'upsert'; key: string; data: T }
  | { type: 'patch'; key: string; data: Partial<T> }
  | { type: 'delete'; key: string };

export type RichUpdate<T> =
  | { type: 'created'; key: string; data: T }
  | { type: 'updated'; key: string; before: T; after: T; patch?: unknown }
  | { type: 'deleted'; key: string; lastKnown?: T };

export interface ViewDef<T, TMode extends 'state' | 'list'> {
  readonly mode: TMode;
  readonly view: string;
  readonly _entity?: T;
}

export interface StackDefinition {
  readonly name: string;
  readonly views: Record<string, ViewGroup>;
  instructions?: Record<string, import('./instructions').InstructionDefinition>;
}

export interface ViewGroup {
  state?: ViewDef<unknown, 'state'>;
  list?: ViewDef<unknown, 'list'>;
}

export interface Subscription {
  view: string;
  key?: string;
  partition?: string;
  filters?: Record<string, string>;
  take?: number;
  skip?: number;
}

export interface HyperStackOptions<TStack extends StackDefinition> {
  stack: TStack;
  autoReconnect?: boolean;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
}

export const DEFAULT_MAX_ENTRIES_PER_VIEW = 10_000;

export interface HyperStackConfig {
  websocketUrl?: string;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  initialSubscriptions?: Subscription[];
  maxEntriesPerView?: number | null;
}

export const DEFAULT_CONFIG: Required<
  Pick<HyperStackConfig, 'reconnectIntervals' | 'maxReconnectAttempts' | 'maxEntriesPerView'>
> = {
  reconnectIntervals: [1000, 2000, 4000, 8000, 16000],
  maxReconnectAttempts: 5,
  maxEntriesPerView: DEFAULT_MAX_ENTRIES_PER_VIEW,
};

export class HyperStackError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: unknown
  ) {
    super(message);
    this.name = 'HyperStackError';
  }
}

export type TypedViews<TViews extends StackDefinition['views']> = {
  [K in keyof TViews]: TypedViewGroup<TViews[K]>;
};

export type TypedViewGroup<TGroup> = {
  state: TGroup extends { state: ViewDef<infer T, 'state'> }
    ? TypedStateView<T>
    : never;
  list: TGroup extends { list: ViewDef<infer T, 'list'> }
    ? TypedListView<T>
    : never;
};

export interface TypedStateView<T> {
  watch(key: string): AsyncIterable<Update<T>>;
  watchRich(key: string): AsyncIterable<RichUpdate<T>>;
  get(key: string): Promise<T | null>;
  getSync(key: string): T | null | undefined;
}

export interface TypedListView<T> {
  watch(): AsyncIterable<Update<T>>;
  watchRich(): AsyncIterable<RichUpdate<T>>;
  get(): Promise<T[]>;
  getSync(): T[] | undefined;
}

export type SubscribeCallback<T> = (update: Update<T>) => void;
export type UnsubscribeFn = () => void;

export type ConnectionStateCallback = (state: ConnectionState, error?: string) => void;

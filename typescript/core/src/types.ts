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
  readonly url: string;
  readonly views: Record<string, ViewGroup>;
  readonly schemas?: Record<string, Schema<unknown>>;
  instructions?: Record<string, import('./instructions').InstructionHandler>;
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
  /** Whether to include initial snapshot (defaults to true for backward compatibility) */
  withSnapshot?: boolean;
  /** Cursor for resuming from a specific point (_seq value) */
  after?: string;
  /** Maximum number of entities to include in snapshot (pagination hint) */
  snapshotLimit?: number;
}

export type SchemaResult<T> =
  | { success: true; data: T }
  | { success: false; error: unknown };

export interface Schema<T> {
  safeParse: (input: unknown) => SchemaResult<T>;
}

export interface WatchOptions<TSchema = unknown> {
  take?: number;
  skip?: number;
  filters?: Record<string, string>;
  schema?: Schema<TSchema>;
  /** Whether to include initial snapshot (defaults to true) */
  withSnapshot?: boolean;
  /** Cursor for resuming from a specific point (_seq value) */
  after?: string;
  /** Maximum number of entities to include in snapshot */
  snapshotLimit?: number;
}

export interface HyperStackOptions<TStack extends StackDefinition> {
  stack: TStack;
  autoReconnect?: boolean;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  validateFrames?: boolean;
}

export const DEFAULT_MAX_ENTRIES_PER_VIEW = 10_000;

/**
 * Authentication configuration for Hyperstack connections
 */
export interface AuthConfig {
  /** Custom token provider function - called before each connection */
  getToken?: () => Promise<string>;
  /** Hyperstack Cloud token endpoint URL */
  tokenEndpoint?: string;
  /** Publishable key for Hyperstack Cloud */
  publishableKey?: string;
  /** Pre-minted static token (for server-side use) */
  token?: string;
}

export interface HyperStackConfig {
  websocketUrl?: string;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  initialSubscriptions?: Subscription[];
  maxEntriesPerView?: number | null;
  /** Authentication configuration */
  auth?: AuthConfig;
}

export const DEFAULT_CONFIG: Required<
  Pick<HyperStackConfig, 'reconnectIntervals' | 'maxReconnectAttempts' | 'maxEntriesPerView'>
> = {
  reconnectIntervals: [1000, 2000, 4000, 8000, 16000],
  maxReconnectAttempts: 5,
  maxEntriesPerView: DEFAULT_MAX_ENTRIES_PER_VIEW,
};

/**
 * Authentication error codes
 */
export type AuthErrorCode =
  | 'AUTH_REQUIRED'
  | 'TOKEN_EXPIRED'
  | 'TOKEN_INVALID'
  | 'QUOTA_EXCEEDED';

export class HyperStackError extends Error {
  constructor(
    message: string,
    public code: string | AuthErrorCode,
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
  [K in keyof TGroup]: TGroup[K] extends ViewDef<infer T, 'state'>
    ? TypedStateView<T>
    : TGroup[K] extends ViewDef<infer T, 'list'>
      ? TypedListView<T>
      : never;
};

export interface TypedStateView<T> {
  use<TSchema = T>(key: string, options?: WatchOptions<TSchema>): AsyncIterable<TSchema>;
  watch(key: string, options?: WatchOptions): AsyncIterable<Update<T>>;
  watchRich(key: string, options?: WatchOptions): AsyncIterable<RichUpdate<T>>;
  get(key: string): Promise<T | null>;
  getSync(key: string): T | null | undefined;
}

export interface TypedListView<T> {
  use<TSchema = T>(options?: WatchOptions<TSchema>): AsyncIterable<TSchema>;
  watch(options?: WatchOptions): AsyncIterable<Update<T>>;
  watchRich(options?: WatchOptions): AsyncIterable<RichUpdate<T>>;
  get(): Promise<T[]>;
  getSync(): T[] | undefined;
}

export type SubscribeCallback<T> = (update: Update<T>) => void;
export type UnsubscribeFn = () => void;

export type ConnectionStateCallback = (state: ConnectionState, error?: string) => void;

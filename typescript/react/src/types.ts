export interface EntityFrame<T = unknown> {
  mode: 'state' | 'append' | 'list';
  entity: string;
  op: 'create' | 'upsert' | 'patch' | 'delete' | 'snapshot';
  key: string;
  data: T;
  append?: string[];
}

export interface SnapshotEntity<T = unknown> {
  key: string;
  data: T;
}

export interface SnapshotFrame<T = unknown> {
  mode: 'state' | 'append' | 'list';
  entity: string;
  op: 'snapshot';
  data: SnapshotEntity<T>[];
}

export type Frame<T = unknown> = EntityFrame<T> | SnapshotFrame<T>;

export function isSnapshotFrame<T>(frame: Frame<T>): frame is SnapshotFrame<T> {
  return frame.op === 'snapshot';
}

export interface Subscription {
  view: string;
  key?: string;
  partition?: string;
  filters?: Record<string, string>;
}

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'error'
  | 'reconnecting';

export interface HyperState<T = unknown> {
  connectionState: ConnectionState;
  lastError?: string;
  entities: Map<string, Map<string, T>>;
  recentFrames: EntityFrame<T>[];
}

export interface HyperSDKConfig {
  websocketUrl?: string;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  initialSubscriptions?: Subscription[];
  autoSubscribeDefault?: boolean;
}

export const DEFAULT_CONFIG: HyperSDKConfig = {
  websocketUrl: 'ws://localhost:8080',
  reconnectIntervals: [1000, 2000, 4000, 8000, 16000],
  maxReconnectAttempts: 5,
  initialSubscriptions: [],
  autoSubscribeDefault: true,
};

export class HyperStreamError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: unknown
  ) {
    super(message);
    this.name = 'HyperStreamError';
  }
}

export type ViewMode = 'state' | 'list';

export interface NetworkConfig {
  name: string;
  websocketUrl: string;
}

export interface ViewDef<T, TMode extends ViewMode> {
  readonly mode: TMode;
  readonly view: string;
  readonly _entity?: T;
}

export interface TransactionDefinition<TParams = unknown> {
  build: (params: TParams) => {
    instruction: string;
    params: TParams;
  };
  refresh?: Array<{ view: string; key?: string | ((params: TParams) => string) }>;
}

export interface StackDefinition {
  readonly name: string;
  readonly views: Record<string, ViewGroup>;
  transactions?: Record<string, TransactionDefinition>;
}

export interface ViewGroup {
  state?: ViewDef<unknown, 'state'>;
  list?: ViewDef<unknown, 'list'>;
}

export interface HyperstackConfig {
  websocketUrl?: string;
  network?: 'devnet' | 'mainnet' | 'localnet' | NetworkConfig;
  apiKey?: string;
  autoConnect?: boolean;
  wallet?: WalletAdapter;
}

export interface WalletAdapter {
  publicKey: string;
  signAndSend: (transaction: unknown) => Promise<string>;
}

export interface ViewHookOptions {
  enabled?: boolean;
  initialData?: unknown;
  refreshOnReconnect?: boolean;
}

export interface ViewHookResult<T> {
  data: T | undefined;
  isLoading: boolean;
  error?: Error;
  refresh: () => void;
}

export interface ListParams {
  key?: string;
  where?: Record<string, unknown>;
  limit?: number;
  filters?: Record<string, string>;
}

export interface UseMutationReturn {
  submit: (instructionOrTx: unknown | unknown[]) => Promise<string>;
  status: 'idle' | 'pending' | 'success' | 'error';
  error?: string;
  signature?: string;
  reset: () => void;
}

export interface StateViewHook<T> {
  use: (key: { [keyField: string]: string }, options?: ViewHookOptions) => ViewHookResult<T>;
}

export interface ListViewHook<T> {
  use: (params?: ListParams, options?: ViewHookOptions) => ViewHookResult<T[]>;
}

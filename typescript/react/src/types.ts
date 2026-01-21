export type {
  ConnectionState,
  Subscription,
  Frame,
  EntityFrame,
  SnapshotFrame,
  SnapshotEntity,
  Update,
  RichUpdate,
} from 'hyperstack-typescript';

export { DEFAULT_MAX_ENTRIES_PER_VIEW } from 'hyperstack-typescript';

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
  /** Allow arbitrary derived views with any name */
  [key: string]: ViewDef<unknown, ViewMode> | undefined;
}

export interface HyperstackConfig {
  websocketUrl?: string;
  network?: 'devnet' | 'mainnet' | 'localnet' | NetworkConfig;
  apiKey?: string;
  autoConnect?: boolean;
  wallet?: WalletAdapter;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  maxEntriesPerView?: number | null;
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

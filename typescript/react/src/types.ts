import type { WalletAdapter } from 'hyperstack-typescript';

export type {
  ConnectionState,
  Subscription,
  Frame,
  EntityFrame,
  SnapshotFrame,
  SnapshotEntity,
  Update,
  RichUpdate,
  StackDefinition,
  ViewDef,
  ViewGroup,
  WalletAdapter,
} from 'hyperstack-typescript';

export { DEFAULT_MAX_ENTRIES_PER_VIEW } from 'hyperstack-typescript';

export type ViewMode = 'state' | 'list';

export interface NetworkConfig {
  name: string;
  websocketUrl: string;
}

export interface TransactionDefinition<TParams = unknown> {
  build: (params: TParams) => {
    instruction: string;
    params: TParams;
  };
  refresh?: Array<{ view: string; key?: string | ((params: TParams) => string) }>;
}

export const DEFAULT_FLUSH_INTERVAL_MS = 16;

export interface HyperstackConfig {
  websocketUrl?: string;
  network?: 'devnet' | 'mainnet' | 'localnet' | NetworkConfig;
  apiKey?: string;
  autoConnect?: boolean;
  wallet?: WalletAdapter;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  maxEntriesPerView?: number | null;
  flushIntervalMs?: number;
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

export interface ListParamsBase {
  key?: string;
  where?: Record<string, unknown>;
  limit?: number;
  filters?: Record<string, string>;
  skip?: number;
}

export interface ListParamsSingle extends ListParamsBase {
  take: 1;
}

export interface ListParamsMultiple extends ListParamsBase {
  take?: number;
}

export type ListParams = ListParamsSingle | ListParamsMultiple;

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
  use(params: ListParamsSingle, options?: ViewHookOptions): ViewHookResult<T | undefined>;
  use(params?: ListParamsMultiple, options?: ViewHookOptions): ViewHookResult<T[]>;
  useOne: (params?: Omit<ListParamsBase, 'take'>, options?: ViewHookOptions) => ViewHookResult<T | undefined>;
}

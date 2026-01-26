import type { PublicKey, Transaction, Connection, TransactionInstruction } from '@solana/web3.js';
import type { Adapter } from '@solana/wallet-adapter-base';

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

export interface TransactionDefinition<TArgs extends any[] = any[]> {
  build: (...args: TArgs) => TransactionInstruction | TransactionInstruction[];
  refresh?: ReadonlyArray<{ view: string; key?: string | ((...args: TArgs) => string) }>;
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

export const DEFAULT_FLUSH_INTERVAL_MS = 16;

export interface HyperstackConfig {
  websocketUrl?: string;
  network?: 'devnet' | 'mainnet' | 'localnet' | NetworkConfig;
  apiKey?: string;
  autoConnect?: boolean;
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  maxEntriesPerView?: number | null;
  rpcUrl?: string;
  connection?: Connection;
  commitment?: 'processed' | 'confirmed' | 'finalized';
  wallets?: Adapter[];
  /**
   * Interval in milliseconds to buffer WebSocket updates before flushing to Zustand.
   * Reduces React re-renders during high-frequency updates.
   * Default: 16ms (one frame at 60fps)
   * Set to 0 for immediate updates (no buffering).
   */
  flushIntervalMs?: number;
}

export interface TransactionOptions {
  skipPreflight?: boolean;
  maxRetries?: number;
  preflightCommitment?: 'processed' | 'confirmed' | 'finalized';
}

export interface WalletAdapter {
  publicKey: PublicKey | null;
  signTransaction<T extends Transaction>(tx: T): Promise<T>;
  signAllTransactions<T extends Transaction>(txs: T[]): Promise<T[]>;
  connected: boolean;
  /** @deprecated Legacy support - use signTransaction instead */
  signAndSend?: (transaction: unknown) => Promise<string>;
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
  submit: (instructionOrTx: TransactionInstruction | TransactionInstruction[]) => Promise<string>;
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

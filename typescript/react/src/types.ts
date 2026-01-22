import type { PublicKey, Transaction, Connection } from '@solana/web3.js';
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
  reconnectIntervals?: number[];
  maxReconnectAttempts?: number;
  maxEntriesPerView?: number | null;
  rpcUrl?: string;
  connection?: Connection;
  commitment?: 'processed' | 'confirmed' | 'finalized';
  wallets?: Adapter[];
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

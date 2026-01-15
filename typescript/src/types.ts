export interface EntityFrame<T = unknown> {
  mode: 'state' | 'append' | 'list';
  entity: string;
  op: 'create' | 'upsert' | 'patch' | 'delete';
  key: string;
  data: T;
}

export interface Subscription {
  view: string;
  key?: string;
  partition?: string;
  filters?: Record<string, string>;
}

// WebSocket connection lifecycle states
export type ConnectionState =
  | 'disconnected'  // not connected, initial state
  | 'connecting'    // attempting to connect
  | 'connected'     // successfully connected and ready
  | 'error'         // connection failed or lost
  | 'reconnecting'; // auto-reconnecting after disconnect


// The core Zustand store state - holds ALL entity data
export interface HyperState<T = unknown> {
  connectionState: ConnectionState;        // current WebSocket state
  lastError?: string;                      // last connection/parsing error message
  entities: Map<string, Map<string, T>>;   // nested maps: entityType -> entityKey -> entityData
  recentFrames: EntityFrame<T>[];          // recent frames for debugging
}

// SDK configuration - customize behavior without code changes
export interface HyperSDKConfig {
  websocketUrl?: string;                      // WebSocket server endpoint
  reconnectIntervals?: number[];              // array of delays in ms for each retry attempt (enables exponential backoff or custom patterns)
  maxReconnectAttempts?: number;              // max reconnections before giving up (defaults to reconnectIntervals.length)
  initialSubscriptions?: Subscription[];      // auto-sent on connect (useful for global data)
  supportsUnsubscribe?: boolean;              // whether server handles unsubscribe messages
  autoSubscribeDefault?: boolean;             // default for hooks auto-subscribe behavior
}

// Sensible defaults - can be overridden per ConnectionManager instance
export const DEFAULT_CONFIG: HyperSDKConfig = {
  websocketUrl: 'ws://localhost:8080',         // default local development server
  reconnectIntervals: [1000, 2000, 4000, 8000, 16000], // exponential backoff: 1s, 2s, 4s, 8s, 16s
  maxReconnectAttempts: 5,                     // retry 5 times before giving up (matches array length)
  initialSubscriptions: [],                    // no global subscriptions by default
  supportsUnsubscribe: false,                  // conservative default - many servers don't support this
  autoSubscribeDefault: true,                  // hooks auto-subscribe by default
};

// Custom error class for SDK-specific errors with structured details
export class HyperStreamError extends Error {
  constructor(
    message: string,                 // human-readable error message
    public code: string,             // machine-readable error code (e.g., 'CONNECTION_FAILED')
    public details?: unknown         // additional context (original error, frame data, etc.)
  ) {
    super(message);
    this.name = 'HyperStreamError';  // Proper error name for debugging/logging
  }
}

export type ViewMode = 'state' | 'list';

export interface NetworkConfig {
  name: string;
  websocketUrl: string;
}

export interface ViewDefinition<T = any> {
  mode: ViewMode;
  view: string;
  type: T;
  transform?: (data: any) => T;
}

export interface TransactionDefinition<TParams = any> {
  build: (params: TParams) => {
    instruction: string;
    params: TParams;
  };
  refresh?: Array<{ view: string; key?: string | ((params: TParams) => string) }>;
}

export interface StackDefinition {
  name: string;
  views: Record<string, any>;
  transactions?: Record<string, TransactionDefinition>;
  helpers?: Record<string, (...args: any[]) => any>;
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
  signAndSend: (transaction: any) => Promise<string>;
}

export interface ViewHookOptions {
  enabled?: boolean;
  initialData?: any;
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
  where?: Record<string, any>;
  limit?: number;
  filters?: Record<string, string>;
}

export interface UseMutationReturn {
  submit: (instructionOrTx: any | any[]) => Promise<string>;
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

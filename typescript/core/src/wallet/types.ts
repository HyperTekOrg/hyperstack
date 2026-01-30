/**
 * Wallet adapter interface for signing and sending transactions.
 * This is framework-agnostic and can be implemented by any wallet provider.
 */
export interface WalletAdapter {
  /** The wallet's public key as a base58-encoded string */
  publicKey: string;

  /**
   * Sign and send a transaction.
   * @param transaction - The transaction to sign and send (can be raw bytes, a Transaction object, or an array of instructions)
   * @returns The transaction signature
   */
  signAndSend: (transaction: unknown) => Promise<string>;
}

/**
 * Wallet connection state
 */
export type WalletState = 'disconnected' | 'connecting' | 'connected' | 'error';

/**
 * Options for wallet connection
 */
export interface WalletConnectOptions {
  /** Whether to use the default wallet selection UI if multiple wallets are available */
  useDefaultSelector?: boolean;
  /** Specific wallet provider to use */
  provider?: string;
}

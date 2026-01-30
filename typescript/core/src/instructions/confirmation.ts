import type { WalletAdapter } from '../wallet/types';
import type { AccountMeta, AccountResolutionOptions } from './account-resolver';

/**
 * Confirmation level for transaction processing.
 * - `processed`: Transaction processed but not confirmed
 * - `confirmed`: Transaction confirmed by cluster
 * - `finalized`: Transaction finalized (recommended for production)
 */
export type ConfirmationLevel = 'processed' | 'confirmed' | 'finalized';

/**
 * Options for executing an instruction.
 */
export interface ExecuteOptions {
  /** Wallet adapter for signing */
  wallet?: WalletAdapter;
  /** User-provided account addresses */
  accounts?: Record<string, string>;
  /** Confirmation level to wait for */
  confirmationLevel?: ConfirmationLevel;
  /** Maximum time to wait for confirmation (ms) */
  timeout?: number;
  /** Refresh view after transaction completes */
  refresh?: {
    view: string;
    key?: string;
  }[];
}

/**
 * Result of a successful instruction execution.
 */
export interface ExecutionResult {
  /** Transaction signature */
  signature: string;
  /** Confirmation level achieved */
  confirmationLevel: ConfirmationLevel;
  /** Slot when transaction was processed */
  slot: number;
  /** Error code if transaction failed */
  error?: string;
}

/**
 * Waits for transaction confirmation.
 * 
 * @param signature - Transaction signature
 * @param level - Desired confirmation level
 * @param timeout - Maximum wait time in milliseconds
 * @returns Confirmation result
 */
export async function waitForConfirmation(
  signature: string,
  level: ConfirmationLevel = 'confirmed',
  timeout: number = 60000
): Promise<{ level: ConfirmationLevel; slot: number }> {
  const startTime = Date.now();
  
  while (Date.now() - startTime < timeout) {
    const status = await checkTransactionStatus(signature);
    
    if (status.err) {
      throw new Error(`Transaction failed: ${JSON.stringify(status.err)}`);
    }
    
    if (isConfirmationLevelSufficient(status.confirmations, level)) {
      return {
        level,
        slot: status.slot,
      };
    }
    
    await sleep(1000);
  }
  
  throw new Error(`Transaction confirmation timeout after ${timeout}ms`);
}

async function checkTransactionStatus(signature: string): Promise<{
  err: unknown;
  confirmations: number | null;
  slot: number;
}> {
  // In production, query the Solana RPC
  return {
    err: null,
    confirmations: 32,
    slot: 123456789,
  };
}

function isConfirmationLevelSufficient(
  confirmations: number | null,
  level: ConfirmationLevel
): boolean {
  if (confirmations === null) {
    return false;
  }
  
  switch (level) {
    case 'processed':
      return confirmations >= 0;
    case 'confirmed':
      return confirmations >= 1;
    case 'finalized':
      return confirmations >= 32;
    default:
      return false;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

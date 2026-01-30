import type { WalletAdapter } from '../wallet/types';

/**
 * Categories of accounts in an instruction.
 * - `signer`: Must sign the transaction (e.g., user wallet)
 * - `known`: Program-derived addresses with fixed known addresses (e.g., System Program)
 * - `pda`: Program-derived addresses computed from seeds
 * - `userProvided`: Must be provided by the caller (e.g., mint, bonding curve)
 */
export type AccountCategory = 'signer' | 'known' | 'pda' | 'userProvided';

/**
 * Metadata for a single account in an instruction.
 */
export interface AccountMeta {
  /** Account name (e.g., "user", "mint") */
  name: string;
  /** Whether this account must sign the transaction */
  isSigner: boolean;
  /** Whether this account is writable */
  isWritable: boolean;
  /** Category of this account */
  category: AccountCategory;
  /** Fixed address for "known" accounts (e.g., "11111111111111111111111111111111") */
  knownAddress?: string;
  /** PDA configuration for "pda" accounts */
  pdaConfig?: PdaConfig;
  /** Whether this account is optional */
  isOptional?: boolean;
}

/**
 * Configuration for PDA (Program-Derived Address) derivation.
 */
export interface PdaConfig {
  /** Program ID that owns this PDA (defaults to instruction's programId) */
  programId?: string;
  /** Seed definitions for PDA derivation */
  seeds: PdaSeed[];
}

/**
 * Single seed in a PDA derivation.
 */
export type PdaSeed =
  | { type: 'literal'; value: string }
  | { type: 'argRef'; argName: string }
  | { type: 'accountRef'; accountName: string };

/**
 * Resolved account with its final address.
 */
export interface ResolvedAccount {
  /** Account name */
  name: string;
  /** The resolved public key address */
  address: string;
  /** Whether this account must sign */
  isSigner: boolean;
  /** Whether this account is writable */
  isWritable: boolean;
}

/**
 * Result of account resolution.
 */
export interface AccountResolutionResult {
  /** All resolved accounts */
  accounts: ResolvedAccount[];
  /** Accounts that need to be provided by the user */
  missingUserAccounts: string[];
}

/**
 * Options for account resolution.
 */
export interface AccountResolutionOptions {
  /** User-provided account addresses */
  accounts?: Record<string, string>;
  /** Wallet adapter for signer accounts */
  wallet?: WalletAdapter;
}

/**
 * Resolves instruction accounts by categorizing and deriving addresses.
 * 
 * @param accountMetas - Account metadata from the instruction definition
 * @param args - Instruction arguments (used for PDA derivation)
 * @param options - Resolution options including wallet and user-provided accounts
 * @returns Resolved accounts and any missing required accounts
 */
export function resolveAccounts(
  accountMetas: AccountMeta[],
  args: Record<string, unknown>,
  options: AccountResolutionOptions
): AccountResolutionResult {
  const resolved: ResolvedAccount[] = [];
  const missing: string[] = [];

  for (const meta of accountMetas) {
    const resolvedAccount = resolveSingleAccount(meta, args, options);
    
    if (resolvedAccount) {
      resolved.push(resolvedAccount);
    } else if (!meta.isOptional) {
      missing.push(meta.name);
    }
  }

  return {
    accounts: resolved,
    missingUserAccounts: missing,
  };
}

function resolveSingleAccount(
  meta: AccountMeta,
  args: Record<string, unknown>,
  options: AccountResolutionOptions
): ResolvedAccount | null {
  switch (meta.category) {
    case 'signer':
      return resolveSignerAccount(meta, options.wallet);
    case 'known':
      return resolveKnownAccount(meta);
    case 'pda':
      return resolvePdaAccount(meta, args);
    case 'userProvided':
      return resolveUserProvidedAccount(meta, options.accounts);
    default:
      return null;
  }
}

function resolveSignerAccount(
  meta: AccountMeta,
  wallet?: WalletAdapter
): ResolvedAccount | null {
  if (!wallet) {
    return null;
  }

  return {
    name: meta.name,
    address: wallet.publicKey,
    isSigner: true,
    isWritable: meta.isWritable,
  };
}

function resolveKnownAccount(meta: AccountMeta): ResolvedAccount | null {
  if (!meta.knownAddress) {
    return null;
  }

  return {
    name: meta.name,
    address: meta.knownAddress,
    isSigner: meta.isSigner,
    isWritable: meta.isWritable,
  };
}

function resolvePdaAccount(
  meta: AccountMeta,
  args: Record<string, unknown>
): ResolvedAccount | null {
  if (!meta.pdaConfig) {
    return null;
  }

  // PDA derivation will be implemented in pda.ts
  // For now, return a placeholder that will be resolved later
  return {
    name: meta.name,
    address: '', // Will be derived
    isSigner: meta.isSigner,
    isWritable: meta.isWritable,
  };
}

function resolveUserProvidedAccount(
  meta: AccountMeta,
  accounts?: Record<string, string>
): ResolvedAccount | null {
  const address = accounts?.[meta.name];
  
  if (!address) {
    return null;
  }

  return {
    name: meta.name,
    address,
    isSigner: meta.isSigner,
    isWritable: meta.isWritable,
  };
}

/**
 * Validates that all required accounts are present.
 * 
 * @param result - Account resolution result
 * @throws Error if any required accounts are missing
 */
export function validateAccountResolution(result: AccountResolutionResult): void {
  if (result.missingUserAccounts.length > 0) {
    throw new Error(
      `Missing required accounts: ${result.missingUserAccounts.join(', ')}`
    );
  }
}

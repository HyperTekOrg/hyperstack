import type { WalletAdapter } from '../wallet/types';
import {
  resolveAccounts,
  validateAccountResolution,
  type AccountMeta,
  type AccountResolutionOptions,
  type ResolvedAccount,
} from './account-resolver';
import { waitForConfirmation, type ExecuteOptions, type ExecutionResult } from './confirmation';
import type { ErrorMetadata } from './error-parser';

/**
 * Resolved accounts map passed to the instruction builder.
 * Keys are account names, values are base58 addresses.
 */
export type ResolvedAccounts = Record<string, string>;

/**
 * The instruction object returned by the handler's build function.
 * This is a framework-agnostic representation that can be converted
 * to @solana/web3.js TransactionInstruction.
 */
export interface BuiltInstruction {
  /** Program ID (base58) */
  programId: string;
  /** Account keys in order */
  keys: Array<{
    pubkey: string;
    isSigner: boolean;
    isWritable: boolean;
  }>;
  /** Serialized instruction data */
  data: Uint8Array;
}

/**
 * Instruction handler from the generated stack SDK.
 * The build() function is generated code that handles serialization.
 */
export interface InstructionHandler {
  /** 
   * Build the instruction with resolved accounts.
   * This is generated code - serialization logic lives here.
   */
  build(args: Record<string, unknown>, accounts: ResolvedAccounts): BuiltInstruction;
  
  /** Account metadata - used by core SDK for resolution */
  accounts: AccountMeta[];
  
  /** Error definitions - used by core SDK for error parsing */
  errors: ErrorMetadata[];
}

/**
 * @deprecated Use InstructionHandler instead. Will be removed in next major version.
 * Legacy instruction definition for backwards compatibility.
 */
export interface InstructionDefinition {
  /** Instruction name */
  name: string;
  /** Program ID (base58) */
  programId: string;
  /** 8-byte discriminator */
  discriminator: Uint8Array;
  /** Account metadata */
  accounts: AccountMeta[];
  /** Argument schema for serialization */
  argsSchema: import('./serializer').ArgSchema[];
  /** Error definitions */
  errors: ErrorMetadata[];
}

/**
 * Converts resolved account array to a map for the builder.
 */
function toResolvedAccountsMap(accounts: ResolvedAccount[]): ResolvedAccounts {
  const map: ResolvedAccounts = {};
  for (const account of accounts) {
    map[account.name] = account.address;
  }
  return map;
}

/**
 * Executes an instruction handler with the given arguments and options.
 * 
 * This is the main function for executing Solana instructions. It handles:
 * 1. Account resolution (signer, PDA, user-provided)
 * 2. Calling the generated build() function
 * 3. Transaction signing and sending
 * 4. Confirmation waiting
 * 
 * @param handler - Instruction handler from generated SDK
 * @param args - Instruction arguments
 * @param options - Execution options
 * @returns Execution result with signature
 */
export async function executeInstruction(
  handler: InstructionHandler,
  args: Record<string, unknown>,
  options: ExecuteOptions = {}
): Promise<ExecutionResult> {
  // Step 1: Resolve accounts using handler's account metadata
  const resolutionOptions: AccountResolutionOptions = {
    accounts: options.accounts,
    wallet: options.wallet,
  };
  
  const resolution = resolveAccounts(
    handler.accounts,
    args,
    resolutionOptions
  );
  
  validateAccountResolution(resolution);
  
  // Step 2: Call generated build() function
  const resolvedAccountsMap = toResolvedAccountsMap(resolution.accounts);
  const instruction = handler.build(args, resolvedAccountsMap);
  
  // Step 3: Build transaction from the built instruction
  const transaction = buildTransaction(instruction);
  
  // Step 4: Sign and send
  if (!options.wallet) {
    throw new Error('Wallet required to sign transaction');
  }
  
  const signature = await options.wallet.signAndSend(transaction);
  
  // Step 5: Wait for confirmation
  const confirmationLevel = options.confirmationLevel ?? 'confirmed';
  const timeout = options.timeout ?? 60000;
  
  const confirmation = await waitForConfirmation(
    signature,
    confirmationLevel,
    timeout
  );
  
  return {
    signature,
    confirmationLevel: confirmation.level,
    slot: confirmation.slot,
  };
}

/**
 * Creates a transaction object from a built instruction.
 * 
 * @param instruction - Built instruction from handler
 * @returns Transaction object ready for signing
 */
function buildTransaction(instruction: BuiltInstruction): unknown {
  // This returns a framework-agnostic transaction representation.
  // The wallet adapter is responsible for converting this to the
  // appropriate format (@solana/web3.js Transaction, etc.)
  return {
    instructions: [{
      programId: instruction.programId,
      keys: instruction.keys,
      data: Array.from(instruction.data),
    }],
  };
}

/**
 * Creates an instruction executor bound to a specific wallet.
 * 
 * @param wallet - Wallet adapter
 * @returns Bound executor function
 */
export function createInstructionExecutor(wallet: WalletAdapter) {
  return {
    execute: async (
      handler: InstructionHandler,
      args: Record<string, unknown>,
      options?: Omit<ExecuteOptions, 'wallet'>
    ) => {
      return executeInstruction(handler, args, {
        ...options,
        wallet,
      });
    },
  };
}

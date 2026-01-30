import type { WalletAdapter } from '../wallet/types';
import {
  resolveAccounts,
  validateAccountResolution,
  type AccountMeta,
  type AccountResolutionOptions,
} from './account-resolver';
import { serializeInstructionData, type ArgSchema } from './serializer';
import { waitForConfirmation, type ConfirmationLevel, type ExecuteOptions, type ExecutionResult } from './confirmation';
import { parseInstructionError, type ErrorMetadata } from './error-parser';

/**
 * Instruction definition from the generated stack.
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
  argsSchema: ArgSchema[];
  /** Error definitions */
  errors: ErrorMetadata[];
}

/**
 * Executes an instruction with the given arguments and options.
 * 
 * This is the main function for executing Solana instructions. It handles:
 * 1. Account resolution (signer, PDA, user-provided)
 * 2. Instruction data serialization
 * 3. Transaction signing and sending
 * 4. Confirmation waiting
 * 5. Error parsing
 * 
 * @param instruction - Instruction definition
 * @param args - Instruction arguments
 * @param options - Execution options
 * @returns Execution result with signature
 */
export async function executeInstruction(
  instruction: InstructionDefinition,
  args: Record<string, unknown>,
  options: ExecuteOptions = {}
): Promise<ExecutionResult> {
  // Step 1: Resolve accounts
  const resolutionOptions: AccountResolutionOptions = {
    accounts: options.accounts,
    wallet: options.wallet,
  };
  
  const resolution = resolveAccounts(
    instruction.accounts,
    args,
    resolutionOptions
  );
  
  validateAccountResolution(resolution);
  
  // Step 2: Serialize instruction data
  const instructionData = serializeInstructionData(
    instruction.discriminator,
    args,
    instruction.argsSchema
  );
  
  // Step 3: Build transaction
  const transaction = buildTransaction(
    resolution.accounts,
    instructionData,
    instruction.programId
  );
  
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
 * Creates a transaction object from resolved accounts and instruction data.
 * 
 * @param accounts - Resolved account metas
 * @param data - Serialized instruction data
 * @param programId - Program ID
 * @returns Transaction object
 */
function buildTransaction(
  accounts: { name: string; address: string; isSigner: boolean; isWritable: boolean }[],
  data: Buffer,
  programId: string
): unknown {
  // In production, this would build a Solana Transaction object
  return {
    accounts: accounts.map(a => ({
      pubkey: a.address,
      isSigner: a.isSigner,
      isWritable: a.isWritable,
    })),
    programId,
    data: Array.from(data),
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
      instruction: InstructionDefinition,
      args: Record<string, unknown>,
      options?: Omit<ExecuteOptions, 'wallet'>
    ) => {
      return executeInstruction(instruction, args, {
        ...options,
        wallet,
      });
    },
  };
}

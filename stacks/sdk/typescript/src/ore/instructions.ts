/**
 * ORE Protocol Instruction Builders
 * 
 * ORE is a proof-of-work mining protocol on Solana where miners deploy SOL
 * to squares on a board and earn ORE tokens as rewards.
 * 
 * Program: oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv
 * Mint: oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp
 */

import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  createInstruction,
  encodeu64,
  encodeu32,
  encodeu8,
  serializeInstructionData,
} from '../utils/instruction-builder.js';

// Program Constants
export const ORE_PROGRAM_ID = new PublicKey(
  'oreV3EG1i9BEgiAJ8b177Z2S2rMarzak4NMv1kULvWv'
);

export const ORE_MINT = new PublicKey(
  'oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp'
);

export const TOKEN_PROGRAM_ID = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
);

export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
);

export const ENTROPY_PROGRAM_ID = new PublicKey(
  '3jSkUuYBoJzQPMEzTvkDFXCZUBksPamrVhrnHR9igu2X'
);

// PDA Seeds
const AUTOMATION_SEED = 'automation';
const MINER_SEED = 'miner';
const TREASURY_SEED = 'treasury';
const BOARD_SEED = 'board';
const CONFIG_SEED = 'config';
const ROUND_SEED = 'round';

// Instruction Discriminators (u8)
const DEPLOY_DISCRIMINATOR = 6;
const CLAIM_SOL_DISCRIMINATOR = 3;
const CLAIM_ORE_DISCRIMINATOR = 4;
const CHECKPOINT_DISCRIMINATOR = 2;

// ============================================================================
// PDA Helpers
// ============================================================================

export function findMinerPDA(authority: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(MINER_SEED, 'utf-8'), authority.toBuffer()],
    ORE_PROGRAM_ID
  );
}

export function findAutomationPDA(authority: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(AUTOMATION_SEED, 'utf-8'), authority.toBuffer()],
    ORE_PROGRAM_ID
  );
}

export function findTreasuryPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(TREASURY_SEED, 'utf-8')],
    ORE_PROGRAM_ID
  );
}

export function findBoardPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BOARD_SEED, 'utf-8')],
    ORE_PROGRAM_ID
  );
}

export function findConfigPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CONFIG_SEED, 'utf-8')],
    ORE_PROGRAM_ID
  );
}

export function findRoundPDA(roundId: bigint): [PublicKey, number] {
  const roundIdBuffer = Buffer.alloc(8);
  roundIdBuffer.writeBigUInt64LE(roundId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from(ROUND_SEED, 'utf-8'), roundIdBuffer],
    ORE_PROGRAM_ID
  );
}

// ============================================================================
// Deploy Instruction
// ============================================================================

export interface DeployInstructionArgs {
  amount: bigint;
  squares: number;
}

export interface DeployInstructionAccounts {
  signer: PublicKey;
  authority: PublicKey;
  automation: PublicKey;
  board: PublicKey;
  config: PublicKey;
  miner: PublicKey;
  round: PublicKey;
  systemProgram?: PublicKey;
  oreProgram?: PublicKey;
  entropyVar: PublicKey;
  entropyProgram?: PublicKey;
}

/**
 * Deploys SOL to selected squares for the current round
 * 
 * @param accounts - Required accounts
 * @param args - amount (SOL in lamports), squares (bitmask)
 */
export function createDeployInstruction(
  accounts: DeployInstructionAccounts,
  args: DeployInstructionArgs
): TransactionInstruction {
  const argsBuffer = Buffer.concat([
    encodeu64(args.amount),
    encodeu32(args.squares),
  ]);

  const data = serializeInstructionData(
    new Uint8Array([DEPLOY_DISCRIMINATOR]),
    argsBuffer
  );

  return createInstruction(
    ORE_PROGRAM_ID,
    [
      { pubkey: accounts.signer, isSigner: true, isWritable: true },
      { pubkey: accounts.authority, isSigner: false, isWritable: true },
      { pubkey: accounts.automation, isSigner: false, isWritable: true },
      { pubkey: accounts.board, isSigner: false, isWritable: true },
      { pubkey: accounts.config, isSigner: false, isWritable: true },
      { pubkey: accounts.miner, isSigner: false, isWritable: true },
      { pubkey: accounts.round, isSigner: false, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: accounts.oreProgram ?? ORE_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.entropyVar, isSigner: false, isWritable: true },
      { pubkey: accounts.entropyProgram ?? ENTROPY_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data
  );
}

// ============================================================================
// Claim SOL Instruction
// ============================================================================

export interface ClaimSolInstructionAccounts {
  signer: PublicKey;
  miner: PublicKey;
  systemProgram?: PublicKey;
}

/**
 * Claims SOL rewards from the miner account
 */
export function createClaimSolInstruction(
  accounts: ClaimSolInstructionAccounts
): TransactionInstruction {
  const data = serializeInstructionData(
    new Uint8Array([CLAIM_SOL_DISCRIMINATOR])
  );

  return createInstruction(
    ORE_PROGRAM_ID,
    [
      { pubkey: accounts.signer, isSigner: true, isWritable: true },
      { pubkey: accounts.miner, isSigner: false, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data
  );
}

// ============================================================================
// Claim ORE Instruction
// ============================================================================

export interface ClaimOreInstructionAccounts {
  signer: PublicKey;
  miner: PublicKey;
  mint?: PublicKey;
  recipient: PublicKey;
  treasury: PublicKey;
  treasuryTokens: PublicKey;
  systemProgram?: PublicKey;
  tokenProgram?: PublicKey;
  associatedTokenProgram?: PublicKey;
}

/**
 * Claims ORE token rewards from the treasury vault
 */
export function createClaimOreInstruction(
  accounts: ClaimOreInstructionAccounts
): TransactionInstruction {
  const data = serializeInstructionData(
    new Uint8Array([CLAIM_ORE_DISCRIMINATOR])
  );

  return createInstruction(
    ORE_PROGRAM_ID,
    [
      { pubkey: accounts.signer, isSigner: true, isWritable: true },
      { pubkey: accounts.miner, isSigner: false, isWritable: true },
      { pubkey: accounts.mint ?? ORE_MINT, isSigner: false, isWritable: false },
      { pubkey: accounts.recipient, isSigner: false, isWritable: true },
      { pubkey: accounts.treasury, isSigner: false, isWritable: true },
      { pubkey: accounts.treasuryTokens, isSigner: false, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: accounts.tokenProgram ?? TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.associatedTokenProgram ?? ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data
  );
}

// ============================================================================
// Checkpoint Instruction
// ============================================================================

export interface CheckpointInstructionAccounts {
  signer: PublicKey;
  board: PublicKey;
  miner: PublicKey;
  round: PublicKey;
  treasury: PublicKey;
  systemProgram?: PublicKey;
}

/**
 * Settles miner rewards for a completed round
 */
export function createCheckpointInstruction(
  accounts: CheckpointInstructionAccounts
): TransactionInstruction {
  const data = serializeInstructionData(
    new Uint8Array([CHECKPOINT_DISCRIMINATOR])
  );

  return createInstruction(
    ORE_PROGRAM_ID,
    [
      { pubkey: accounts.signer, isSigner: true, isWritable: true },
      { pubkey: accounts.board, isSigner: false, isWritable: false },
      { pubkey: accounts.miner, isSigner: false, isWritable: true },
      { pubkey: accounts.round, isSigner: false, isWritable: true },
      { pubkey: accounts.treasury, isSigner: false, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data
  );
}

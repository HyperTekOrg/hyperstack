/**
 * PumpFun Protocol Instruction Builders
 * 
 * This module provides builders for constructing PumpFun protocol transactions.
 * PumpFun is a bonding curve-based token launch platform on Solana where:
 * - Tokens are created with a bonding curve that determines price based on supply
 * - Users can buy tokens with SOL, increasing the price along the curve
 * - Users can sell tokens back for SOL, decreasing the price
 * - When the curve completes, liquidity migrates to Raydium DEX
 * 
 * Architecture:
 * - Each token has a BondingCurve account (PDA) that tracks reserves and state
 * - Associated token accounts hold the curve's token reserves
 * - Creator vaults receive a portion of fees (if creator is set)
 * - Volume accumulators track user and global trading metrics
 * 
 * References:
 * - IDL: stacks/pumpfun/idl/pump.json
 * - Program: 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P
 * - Fee Program: pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ
 */

import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from '@solana/web3.js';
import {
  createInstruction,
  encodeu64,
  encodeOption,
  encodeBoolean,
  serializeInstructionData,
} from '../utils/instruction-builder.js';

// ============================================================================
// Program Constants
// ============================================================================

/**
 * PumpFun program ID
 * This is the main program that handles all bonding curve operations.
 */
export const PUMP_PROGRAM_ID = new PublicKey(
  '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P'
);

/**
 * PumpFun fee program ID
 * Handles fee configuration and distribution logic.
 */
export const PUMP_FEE_PROGRAM_ID = new PublicKey(
  'pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ'
);

/**
 * SPL Token program ID
 * Standard Solana token program for token operations.
 */
export const TOKEN_PROGRAM_ID = new PublicKey(
  'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
);

/**
 * SPL Associated Token Account program ID
 * Used for deriving associated token accounts (ATAs).
 */
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
);

// ============================================================================
// PDA Seeds (as UTF-8 strings for clarity)
// ============================================================================

/**
 * Seed for deriving the global state PDA
 * The global account stores protocol-wide configuration like fee recipients.
 */
const GLOBAL_SEED = 'global';

/**
 * Seed for deriving bonding curve PDAs
 * Each token has a unique bonding curve account: [b"bonding-curve", mint_pubkey]
 */
const BONDING_CURVE_SEED = 'bonding-curve';

/**
 * Seed for deriving creator vault PDAs
 * Creator vaults receive a portion of trading fees: [b"creator-vault", creator_pubkey]
 */
const CREATOR_VAULT_SEED = 'creator-vault';

/**
 * Seed for deriving event authority PDA
 * Used for emitting program events via CPI.
 */
const EVENT_AUTHORITY_SEED = '__event_authority';

/**
 * Seed for deriving global volume accumulator PDA
 * Tracks aggregate trading volume across all tokens.
 */
const GLOBAL_VOLUME_ACCUMULATOR_SEED = 'global_volume_accumulator';

/**
 * Seed for deriving user volume accumulator PDAs
 * Tracks per-user trading volume: [b"user_volume_accumulator", user_pubkey]
 */
const USER_VOLUME_ACCUMULATOR_SEED = 'user_volume_accumulator';

/**
 * Seed for deriving fee config PDA
 * Stores fee percentages and distribution rules.
 */
const FEE_CONFIG_SEED = 'fee_config';

/**
 * Fee config discriminator (from IDL line 715-748)
 * This is a 32-byte constant used as a seed for fee_config PDA derivation.
 */
const FEE_CONFIG_DISCRIMINATOR = new Uint8Array([
  1, 86, 224, 246, 147, 102, 90, 207, 68, 219, 21, 104, 191, 23, 91, 170, 81,
  137, 203, 151, 245, 210, 255, 59, 101, 93, 43, 182, 253, 109, 24, 176,
]);

// ============================================================================
// Instruction Discriminators (8-byte identifiers from IDL)
// ============================================================================

/**
 * Discriminator for the 'buy' instruction
 * Source: IDL line 426-433
 * Used when buying tokens from the bonding curve with exact token amount.
 */
const BUY_DISCRIMINATOR = new Uint8Array([102, 6, 61, 18, 1, 218, 235, 234]);

/**
 * Discriminator for the 'sell' instruction
 * Source: IDL line 3585-3592
 * Used when selling tokens back to the bonding curve for SOL.
 */
const SELL_DISCRIMINATOR = new Uint8Array([51, 230, 133, 164, 1, 127, 131, 173]);

// ============================================================================
// PDA Derivation Helpers
// ============================================================================

/**
 * Derives the global state PDA
 * 
 * The global account stores protocol configuration:
 * - Fee recipients
 * - Authority addresses for various operations
 * - Protocol-wide settings
 * 
 * PDA derivation: [b"global"]
 * 
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findGlobalPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_SEED, 'utf-8')],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the bonding curve PDA for a given token mint
 * 
 * The bonding curve account stores:
 * - Virtual SOL reserves (for pricing calculations)
 * - Virtual token reserves (for pricing calculations)
 * - Real SOL reserves (actual balance)
 * - Real token reserves (actual balance)
 * - Creator address (receives fee share)
 * - Complete flag (whether curve has graduated to Raydium)
 * 
 * PDA derivation: [b"bonding-curve", mint_pubkey]
 * Source: IDL lines 458-475
 * 
 * @param mint - The token mint public key
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findBondingCurvePDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BONDING_CURVE_SEED, 'utf-8'), mint.toBuffer()],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the associated token account (ATA) for the bonding curve
 * 
 * This is where the bonding curve holds its token reserves.
 * Uses the standard SPL Associated Token Account derivation.
 * 
 * PDA derivation: [bonding_curve_pubkey, token_program_id, mint_pubkey]
 * Program: Associated Token Program (ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL)
 * Source: IDL lines 476-509
 * 
 * @param bondingCurve - The bonding curve PDA
 * @param mint - The token mint public key
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findAssociatedBondingCurveATA(
  bondingCurve: PublicKey,
  mint: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      bondingCurve.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
    ],
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
}

/**
 * Derives the creator vault PDA for a given creator
 * 
 * Creator vaults receive a portion of trading fees if a creator is set.
 * The fee split is typically:
 * - Protocol fee: Goes to global fee_recipient
 * - Creator fee: Goes to creator_vault (then claimable by creator)
 * 
 * PDA derivation: [b"creator-vault", creator_pubkey]
 * Source: IDL lines 565-584
 * 
 * @param creator - The creator's public key
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findCreatorVaultPDA(creator: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CREATOR_VAULT_SEED, 'utf-8'), creator.toBuffer()],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the event authority PDA
 * 
 * Used by the program to emit events via CPI to itself.
 * This is an Anchor pattern for self-invoked CPIs to emit events.
 * 
 * PDA derivation: [b"__event_authority"]
 * Source: IDL lines 592-606
 * 
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findEventAuthorityPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(EVENT_AUTHORITY_SEED, 'utf-8')],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the global volume accumulator PDA
 * 
 * Tracks aggregate trading volume across all tokens in the protocol.
 * Used for analytics and potentially for fee tier calculations.
 * 
 * PDA derivation: [b"global_volume_accumulator"]
 * Source: IDL lines 630-649
 * 
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findGlobalVolumeAccumulatorPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(GLOBAL_VOLUME_ACCUMULATOR_SEED, 'utf-8')],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the user volume accumulator PDA for a given user
 * 
 * Tracks per-user trading volume across all tokens.
 * Can be used for:
 * - Fee tier discounts based on volume
 * - User analytics
 * - Loyalty programs
 * 
 * PDA derivation: [b"user_volume_accumulator", user_pubkey]
 * Source: IDL lines 665-704
 * 
 * @param user - The user's public key
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findUserVolumeAccumulatorPDA(
  user: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(USER_VOLUME_ACCUMULATOR_SEED, 'utf-8'), user.toBuffer()],
    PUMP_PROGRAM_ID
  );
}

/**
 * Derives the fee config PDA
 * 
 * Stores fee configuration including:
 * - Protocol fee basis points
 * - Creator fee basis points
 * - Fee recipient addresses
 * 
 * PDA derivation: [b"fee_config", discriminator_bytes]
 * Program: Fee Program (pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ)
 * Source: IDL lines 707-767
 * 
 * @returns Tuple of [PublicKey, bump seed]
 */
export function findFeeConfigPDA(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(FEE_CONFIG_SEED, 'utf-8'),
      Buffer.from(FEE_CONFIG_DISCRIMINATOR),
    ],
    PUMP_FEE_PROGRAM_ID
  );
}

// ============================================================================
// Instruction Builder: Buy
// ============================================================================

/**
 * Arguments for the buy instruction
 */
export interface BuyInstructionArgs {
  /**
   * Amount of tokens to buy
   * Specified in token's base units (not decimals adjusted)
   */
  amount: bigint;

  /**
   * Maximum amount of SOL willing to spend (slippage protection)
   * Specified in lamports (1 SOL = 1e9 lamports)
   * Transaction will fail if the actual cost exceeds this amount
   */
  maxSolCost: bigint;

  /**
   * Whether to track this trade in volume accumulators
   * - Some(true): Update user and global volume accumulators
   * - Some(false): Don't track volume (saves compute)
   * - None: Use default behavior (typically tracks)
   * 
   * Volume tracking affects:
   * - Fee tier calculations
   * - Analytics
   * - Potential loyalty rewards
   */
  trackVolume?: boolean;
}

/**
 * Accounts required for the buy instruction
 */
export interface BuyInstructionAccounts {
  /**
   * Global protocol state account (PDA: ["global"])
   * Contains fee recipients and protocol configuration
   */
  global: PublicKey;

  /**
   * Fee recipient account (receives protocol fees)
   * Address is stored in the global state
   */
  feeRecipient: PublicKey;

  /**
   * Token mint being traded
   */
  mint: PublicKey;

  /**
   * Bonding curve account for this token (PDA: ["bonding-curve", mint])
   * Contains pricing curve state and reserves
   */
  bondingCurve: PublicKey;

  /**
   * Bonding curve's associated token account (holds token reserves)
   * PDA: [bonding_curve, token_program, mint] under ATA program
   */
  associatedBondingCurve: PublicKey;

  /**
   * User's associated token account (receives purchased tokens)
   * Should be initialized before calling this instruction
   */
  associatedUser: PublicKey;

  /**
   * User's wallet (signer, pays SOL and gas)
   */
  user: PublicKey;

  /**
   * System program (for SOL transfers)
   */
  systemProgram?: PublicKey;

  /**
   * SPL Token program (for token transfers)
   */
  tokenProgram?: PublicKey;

  /**
   * Creator vault (receives creator fees if creator is set)
   * PDA: ["creator-vault", bonding_curve.creator]
   */
  creatorVault: PublicKey;

  /**
   * Event authority (for emitting events)
   * PDA: ["__event_authority"]
   */
  eventAuthority: PublicKey;

  /**
   * Program address (self-reference for CPI)
   */
  program?: PublicKey;

  /**
   * Global volume accumulator (tracks aggregate volume)
   * PDA: ["global_volume_accumulator"]
   */
  globalVolumeAccumulator: PublicKey;

  /**
   * User volume accumulator (tracks user's volume)
   * PDA: ["user_volume_accumulator", user]
   */
  userVolumeAccumulator: PublicKey;

  /**
   * Fee config account (stores fee percentages)
   * PDA: ["fee_config", discriminator] under fee program
   */
  feeConfig: PublicKey;

  /**
   * Fee program (handles fee logic)
   */
  feeProgram?: PublicKey;
}

/**
 * Creates a 'buy' instruction for purchasing tokens from a bonding curve
 * 
 * Flow:
 * 1. User specifies how many tokens they want to buy
 * 2. Program calculates SOL cost based on bonding curve formula:
 *    - Curve uses constant product formula: x * y = k
 *    - Price increases as more tokens are bought
 * 3. SOL is deducted from user (up to maxSolCost)
 * 4. Fees are split:
 *    - Protocol fee → fee_recipient
 *    - Creator fee → creator_vault (if creator exists)
 * 5. Tokens are transferred to user's ATA
 * 6. Curve state is updated (reserves decrease)
 * 7. Volume accumulators are updated (if trackVolume is true)
 * 
 * Pricing formula (from IDL docs):
 * - Virtual reserves determine the price
 * - Real reserves track actual balances
 * - Fees are calculated as basis points (e.g., 100 bps = 1%)
 * 
 * Example:
 * ```ts
 * const [bondingCurve] = findBondingCurvePDA(mint);
 * const [associatedBondingCurve] = findAssociatedBondingCurveATA(bondingCurve, mint);
 * // ... derive other PDAs
 * 
 * const instruction = createBuyInstruction(
 *   {
 *     global,
 *     feeRecipient,
 *     mint,
 *     bondingCurve,
 *     associatedBondingCurve,
 *     associatedUser: userAta,
 *     user: wallet.publicKey,
 *     creatorVault,
 *     eventAuthority,
 *     globalVolumeAccumulator,
 *     userVolumeAccumulator,
 *     feeConfig,
 *   },
 *   {
 *     amount: 1000000n, // 1 token (6 decimals)
 *     maxSolCost: 5000000000n, // 5 SOL max
 *     trackVolume: true,
 *   }
 * );
 * ```
 * 
 * Source: IDL lines 420-790
 * 
 * @param accounts - Account addresses required for the instruction
 * @param args - Instruction arguments (amount, maxSolCost, trackVolume)
 * @returns TransactionInstruction ready to be added to a transaction
 */
export function createBuyInstruction(
  accounts: BuyInstructionAccounts,
  args: BuyInstructionArgs
): TransactionInstruction {
  // Encode instruction arguments following Borsh format
  // Layout: [discriminator:8][amount:u64][max_sol_cost:u64][track_volume:Option<bool>]
  const argsBuffer = Buffer.concat([
    encodeu64(args.amount),
    encodeu64(args.maxSolCost),
    // Rust Option<bool> encoding:
    // - None: [0x00]
    // - Some(false): [0x01, 0x00]
    // - Some(true): [0x01, 0x01]
    encodeOption(args.trackVolume, encodeBoolean),
  ]);

  const data = serializeInstructionData(new Uint8Array(BUY_DISCRIMINATOR), argsBuffer);

  // Build account metas in the exact order specified by the IDL (lines 434-773)
  return createInstruction(
    PUMP_PROGRAM_ID,
    [
      { pubkey: accounts.global, isSigner: false, isWritable: false },
      { pubkey: accounts.feeRecipient, isSigner: false, isWritable: true },
      { pubkey: accounts.mint, isSigner: false, isWritable: false },
      { pubkey: accounts.bondingCurve, isSigner: false, isWritable: true },
      { pubkey: accounts.associatedBondingCurve, isSigner: false, isWritable: true },
      { pubkey: accounts.associatedUser, isSigner: false, isWritable: true },
      { pubkey: accounts.user, isSigner: true, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: accounts.tokenProgram ?? TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.creatorVault, isSigner: false, isWritable: true },
      { pubkey: accounts.eventAuthority, isSigner: false, isWritable: false },
      { pubkey: accounts.program ?? PUMP_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.globalVolumeAccumulator, isSigner: false, isWritable: false },
      { pubkey: accounts.userVolumeAccumulator, isSigner: false, isWritable: true },
      { pubkey: accounts.feeConfig, isSigner: false, isWritable: false },
      { pubkey: accounts.feeProgram ?? PUMP_FEE_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data
  );
}

// ============================================================================
// Instruction Builder: Sell
// ============================================================================

/**
 * Arguments for the sell instruction
 */
export interface SellInstructionArgs {
  /**
   * Amount of tokens to sell
   * Specified in token's base units (not decimals adjusted)
   */
  amount: bigint;

  /**
   * Minimum amount of SOL expected to receive (slippage protection)
   * Specified in lamports (1 SOL = 1e9 lamports)
   * Transaction will fail if the actual output is less than this amount
   */
  minSolOutput: bigint;
}

/**
 * Accounts required for the sell instruction
 */
export interface SellInstructionAccounts {
  /**
   * Global protocol state account (PDA: ["global"])
   */
  global: PublicKey;

  /**
   * Fee recipient account (receives protocol fees)
   */
  feeRecipient: PublicKey;

  /**
   * Token mint being traded
   */
  mint: PublicKey;

  /**
   * Bonding curve account for this token (PDA: ["bonding-curve", mint])
   */
  bondingCurve: PublicKey;

  /**
   * Bonding curve's associated token account (receives sold tokens)
   * PDA: [bonding_curve, token_program, mint] under ATA program
   */
  associatedBondingCurve: PublicKey;

  /**
   * User's associated token account (tokens are burned from here)
   */
  associatedUser: PublicKey;

  /**
   * User's wallet (signer, receives SOL)
   */
  user: PublicKey;

  /**
   * System program (for SOL transfers)
   */
  systemProgram?: PublicKey;

  /**
   * Creator vault (receives creator fees if creator is set)
   * PDA: ["creator-vault", bonding_curve.creator]
   */
  creatorVault: PublicKey;

  /**
   * SPL Token program (for token transfers)
   */
  tokenProgram?: PublicKey;

  /**
   * Event authority (for emitting events)
   * PDA: ["__event_authority"]
   */
  eventAuthority: PublicKey;

  /**
   * Program address (self-reference for CPI)
   */
  program?: PublicKey;

  /**
   * Fee config account (stores fee percentages)
   * PDA: ["fee_config", discriminator] under fee program
   */
  feeConfig: PublicKey;

  /**
   * Fee program (handles fee logic)
   */
  feeProgram?: PublicKey;
}

/**
 * Creates a 'sell' instruction for selling tokens back to a bonding curve
 * 
 * Flow:
 * 1. User specifies how many tokens they want to sell
 * 2. Program calculates SOL output based on bonding curve formula:
 *    - Curve uses constant product formula: x * y = k
 *    - Price decreases as more tokens are sold
 * 3. Tokens are transferred from user's ATA to bonding curve
 * 4. Fees are deducted from SOL output:
 *    - Protocol fee → fee_recipient
 *    - Creator fee → creator_vault (if creator exists)
 * 5. Remaining SOL is transferred to user
 * 6. Curve state is updated (reserves increase)
 * 7. Transaction fails if SOL output < minSolOutput (slippage protection)
 * 
 * Pricing mechanics:
 * - Selling moves price DOWN the bonding curve
 * - Earlier sellers get better prices (more SOL per token)
 * - Large sells have price impact (slippage)
 * - Fees reduce the net SOL received
 * 
 * Example:
 * ```ts
 * const [bondingCurve] = findBondingCurvePDA(mint);
 * const [associatedBondingCurve] = findAssociatedBondingCurveATA(bondingCurve, mint);
 * // ... derive other PDAs
 * 
 * const instruction = createSellInstruction(
 *   {
 *     global,
 *     feeRecipient,
 *     mint,
 *     bondingCurve,
 *     associatedBondingCurve,
 *     associatedUser: userAta,
 *     user: wallet.publicKey,
 *     creatorVault,
 *     eventAuthority,
 *     feeConfig,
 *   },
 *   {
 *     amount: 1000000n, // Sell 1 token (6 decimals)
 *     minSolOutput: 4000000000n, // Expect at least 4 SOL
 *   }
 * );
 * ```
 * 
 * Source: IDL lines 3578-3863
 * 
 * @param accounts - Account addresses required for the instruction
 * @param args - Instruction arguments (amount, minSolOutput)
 * @returns TransactionInstruction ready to be added to a transaction
 */
export function createSellInstruction(
  accounts: SellInstructionAccounts,
  args: SellInstructionArgs
): TransactionInstruction {
  // Encode instruction arguments following Borsh format
  // Layout: [discriminator:8][amount:u64][min_sol_output:u64]
  const argsBuffer = Buffer.concat([
    encodeu64(args.amount),
    encodeu64(args.minSolOutput),
  ]);

  const data = serializeInstructionData(new Uint8Array(SELL_DISCRIMINATOR), argsBuffer);

  // Build account metas in the exact order specified by the IDL (lines 3594-3856)
  return createInstruction(
    PUMP_PROGRAM_ID,
    [
      { pubkey: accounts.global, isSigner: false, isWritable: false },
      { pubkey: accounts.feeRecipient, isSigner: false, isWritable: true },
      { pubkey: accounts.mint, isSigner: false, isWritable: false },
      { pubkey: accounts.bondingCurve, isSigner: false, isWritable: true },
      { pubkey: accounts.associatedBondingCurve, isSigner: false, isWritable: true },
      { pubkey: accounts.associatedUser, isSigner: false, isWritable: true },
      { pubkey: accounts.user, isSigner: true, isWritable: true },
      { pubkey: accounts.systemProgram ?? SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: accounts.creatorVault, isSigner: false, isWritable: true },
      { pubkey: accounts.tokenProgram ?? TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.eventAuthority, isSigner: false, isWritable: false },
      { pubkey: accounts.program ?? PUMP_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: accounts.feeConfig, isSigner: false, isWritable: false },
      { pubkey: accounts.feeProgram ?? PUMP_FEE_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data
  );
}

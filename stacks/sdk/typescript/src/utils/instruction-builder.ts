/**
 * Instruction Builder Utilities
 * 
 * Core utilities for creating Solana TransactionInstructions from IDL specifications.
 * Provides serialization helpers following Anchor's Borsh encoding format.
 */

import {
  TransactionInstruction,
  PublicKey,
} from '@solana/web3.js';

/**
 * Account metadata for a Solana instruction
 */
export interface InstructionAccount {
  /** Account public key */
  pubkey: PublicKey;
  /** Whether this account must sign the transaction */
  isSigner: boolean;
  /** Whether this account's data will be modified */
  isWritable: boolean;
}

/**
 * Creates a Solana TransactionInstruction
 * 
 * @param programId - The program ID that will execute this instruction
 * @param keys - Array of account metadata
 * @param data - Serialized instruction data (discriminator + args)
 * @returns A TransactionInstruction ready to be added to a transaction
 * 
 * @example
 * ```typescript
 * const instruction = createInstruction(
 *   new PublicKey('ProgramId...'),
 *   [
 *     { pubkey: userPubkey, isSigner: true, isWritable: true },
 *     { pubkey: accountPubkey, isSigner: false, isWritable: true }
 *   ],
 *   serializeInstructionData([0x66, 0x06, 0x3d, 0x12, 0x01, 0x03, 0x28, 0xc5], args)
 * );
 * ```
 */
export function createInstruction(
  programId: PublicKey,
  keys: InstructionAccount[],
  data: Buffer
): TransactionInstruction {
  return new TransactionInstruction({
    keys,
    programId,
    data,
  });
}

/**
 * Serializes instruction data following Anchor format:
 * [8-byte discriminator][Borsh-encoded args]
 * 
 * @param discriminator - 8-byte array uniquely identifying the instruction
 * @param args - Borsh-encoded arguments buffer (or undefined if no args)
 * @returns Buffer containing discriminator + args ready for instruction data
 * 
 * @example
 * ```typescript
 * // Instruction with no arguments
 * const data = serializeInstructionData(new Uint8Array([0x66, 0x06, 0x3d, 0x12, 0x01, 0x03, 0x28, 0xc5]));
 * 
 * // Instruction with Borsh-encoded arguments
 * const argsBuffer = Buffer.concat([encodeu64(amount), encodeu64(maxCost)]);
 * const data = serializeInstructionData(new Uint8Array([0x66, 0x06, 0x3d, 0x12, 0x01, 0x03, 0x28, 0xc5]), argsBuffer);
 * ```
 */
export function serializeInstructionData(
  discriminator: Uint8Array | number[],
  args?: Buffer
): Buffer {
  const discArray = discriminator instanceof Uint8Array ? Array.from(discriminator) : discriminator;
  if (discArray.length !== 8) {
    throw new Error(`Discriminator must be exactly 8 bytes, got ${discArray.length}`);
  }

  const discriminatorBuffer = Buffer.from(discArray);
  
  if (!args || args.length === 0) {
    return discriminatorBuffer;
  }

  return Buffer.concat([discriminatorBuffer, args]);
}

/**
 * Encodes a u64 (unsigned 64-bit integer) in little-endian format
 * 
 * @param value - The number to encode (must be non-negative and fit in u64)
 * @returns 8-byte Buffer containing the little-endian representation
 * 
 * @example
 * ```typescript
 * const amount = encodeu64(1_000_000); // 1 million
 * const lamports = encodeu64(100_000_000n); // 0.1 SOL as bigint
 * ```
 */
export function encodeu64(value: number | bigint): Uint8Array {
  const bn = BigInt(value);
  if (bn < 0n) {
    throw new Error('Value must be non-negative');
  }
  if (bn > 0xFFFFFFFFFFFFFFFFn) {
    throw new Error('Value exceeds u64 maximum');
  }

  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(bn);
  return buffer;
}

/**
 * Encodes a u8 (unsigned 8-bit integer)
 * 
 * @param value - The number to encode (0-255)
 * @returns 1-byte Buffer
 */
export function encodeu8(value: number): Uint8Array {
  if (value < 0 || value > 255) {
    throw new Error('Value must be between 0 and 255');
  }
  const buffer = Buffer.alloc(1);
  buffer.writeUInt8(value);
  return buffer;
}

/**
 * Encodes a u16 (unsigned 16-bit integer) in little-endian format
 * 
 * @param value - The number to encode (0-65535)
 * @returns 2-byte Buffer
 */
export function encodeu16(value: number): Uint8Array {
  if (value < 0 || value > 65535) {
    throw new Error('Value must be between 0 and 65535');
  }
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16LE(value);
  return buffer;
}

/**
 * Encodes a u32 (unsigned 32-bit integer) in little-endian format
 * 
 * @param value - The number to encode (0-4294967295)
 * @returns 4-byte Buffer
 */
export function encodeu32(value: number): Uint8Array {
  if (value < 0 || value > 4294967295) {
    throw new Error('Value must be between 0 and 4294967295');
  }
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32LE(value);
  return buffer;
}

/**
 * Encodes a boolean value (1 byte: 0 or 1)
 * 
 * @param value - The boolean to encode
 * @returns 1-byte Buffer (0x00 for false, 0x01 for true)
 */
export function encodeBoolean(value: boolean): Uint8Array {
  return encodeu8(value ? 1 : 0);
}

/**
 * Encodes a PublicKey (32 bytes)
 * 
 * @param pubkey - The PublicKey to encode
 * @returns 32-byte Buffer containing the public key
 */
export function encodePublicKey(pubkey: PublicKey): Uint8Array {
  return Buffer.from(pubkey.toBytes());
}

/**
 * Encodes a UTF-8 string with length prefix (Rust String format)
 * Format: [4-byte length (u32)][UTF-8 bytes]
 * 
 * @param str - The string to encode
 * @returns Buffer with length-prefixed UTF-8 string
 */
export function encodeString(str: string): Buffer {
  const utf8 = Buffer.from(str, 'utf-8');
  const length = encodeu32(utf8.length);
  return Buffer.concat([length, utf8]);
}

/**
 * Encodes an optional value (Rust Option<T> format)
 * Format: [1-byte discriminator (0=None, 1=Some)][value if Some]
 * 
 * @param value - The value to encode, or null/undefined for None
 * @param encoder - Function to encode the value if present
 * @returns Buffer with option discriminator + optional value
 * 
 * @example
 * ```typescript
 * const someValue = encodeOption(1000, encodeu64);
 * const noneValue = encodeOption(null, encodeu64);
 * ```
 */
export function encodeOption<T>(
  value: T | null | undefined,
  encoder: (val: T) => Uint8Array
): Uint8Array {
  if (value === null || value === undefined) {
    return encodeu8(0); // None
  }
  return Buffer.concat([encodeu8(1), encoder(value)]); // Some
}

/**
 * Encodes a vector/array (Rust Vec<T> format)
 * Format: [4-byte length (u32)][element1][element2]...
 * 
 * @param items - Array of items to encode
 * @param encoder - Function to encode each item
 * @returns Buffer with length-prefixed array
 * 
 * @example
 * ```typescript
 * const pubkeys = [pubkey1, pubkey2, pubkey3];
 * const encoded = encodeVec(pubkeys, encodePublicKey);
 * ```
 */
export function encodeVec<T>(
  items: T[],
  encoder: (item: T) => Uint8Array
): Uint8Array {
  const length = encodeu32(items.length);
  const encodedItems = items.map(encoder);
  return Buffer.concat([length, ...encodedItems]);
}

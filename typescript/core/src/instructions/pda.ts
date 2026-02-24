/**
 * PDA (Program Derived Address) derivation utilities.
 * 
 * Implements Solana's PDA derivation algorithm without depending on @solana/web3.js.
 */

import { sha256 } from '@noble/hashes/sha2.js';
import { ed25519 } from '@noble/curves/ed25519.js';

// Base58 alphabet (Bitcoin/Solana style)
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

/**
 * Decode base58 string to Uint8Array.
 */
export function decodeBase58(str: string): Uint8Array {
  if (str.length === 0) {
    return new Uint8Array(0);
  }

  const bytes: number[] = [0];
  
  for (const char of str) {
    const value = BASE58_ALPHABET.indexOf(char);
    if (value === -1) {
      throw new Error('Invalid base58 character: ' + char);
    }
    
    let carry = value;
    for (let i = 0; i < bytes.length; i++) {
      carry += (bytes[i] ?? 0) * 58;
      bytes[i] = carry & 0xff;
      carry >>= 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }
  
  // Add leading zeros for each leading '1' in input
  for (const char of str) {
    if (char !== '1') break;
    bytes.push(0);
  }
  
  return new Uint8Array(bytes.reverse());
}

/**
 * Encode Uint8Array to base58 string.
 */
export function encodeBase58(bytes: Uint8Array): string {
  if (bytes.length === 0) {
    return '';
  }

  const digits: number[] = [0];
  
  for (const byte of bytes) {
    let carry = byte;
    for (let i = 0; i < digits.length; i++) {
      carry += (digits[i] ?? 0) << 8;
      digits[i] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = (carry / 58) | 0;
    }
  }
  
  // Add leading zeros for each leading 0 byte in input
  for (const byte of bytes) {
    if (byte !== 0) break;
    digits.push(0);
  }
  
  return digits.reverse().map(d => BASE58_ALPHABET[d]).join('');
}

/**
 * SHA-256 hash function (synchronous).
 */
function sha256Sync(data: Uint8Array): Uint8Array {
  return sha256(data);
}

/**
 * SHA-256 hash function (async, works in browser and Node.js).
 */
async function sha256Async(data: Uint8Array): Promise<Uint8Array> {
  if (typeof globalThis !== 'undefined' && globalThis.crypto && globalThis.crypto.subtle) {
    // Create a copy of the data to ensure we have an ArrayBuffer
    const copy = new Uint8Array(data);
    const hashBuffer = await globalThis.crypto.subtle.digest('SHA-256', copy);
    return new Uint8Array(hashBuffer);
  }
  return sha256Sync(data);
}

/**
 * Check if a point is on the ed25519 curve.
 * A valid PDA must be OFF the curve to ensure it has no corresponding private key.
 * Uses @noble/curves for browser-compatible ed25519 validation.
 */
function isOnCurve(publicKey: Uint8Array): boolean {
  try {
    // Try to decode as an ed25519 point - if successful, it's on the curve
    ed25519.Point.fromBytes(publicKey);
    return true; // Point is on curve - invalid for PDA
  } catch {
    return false; // Point is off curve - valid for PDA
  }
}

/**
 * PDA marker bytes appended to seeds before hashing.
 */
const PDA_MARKER = new TextEncoder().encode('ProgramDerivedAddress');

/**
 * Build the hash input buffer for PDA derivation.
 */
function buildPdaBuffer(
  seeds: Uint8Array[],
  programIdBytes: Uint8Array,
  bump: number
): Uint8Array {
  const totalLength = seeds.reduce((sum, s) => sum + s.length, 0) 
    + 1 // bump
    + 32 // programId
    + PDA_MARKER.length;
  
  const buffer = new Uint8Array(totalLength);
  let offset = 0;
  
  // Copy seeds
  for (const seed of seeds) {
    buffer.set(seed, offset);
    offset += seed.length;
  }
  
  // Add bump seed
  buffer[offset++] = bump;
  
  // Add program ID
  buffer.set(programIdBytes, offset);
  offset += 32;
  
  // Add PDA marker
  buffer.set(PDA_MARKER, offset);
  
  return buffer;
}

/**
 * Validate seeds before PDA derivation.
 */
function validateSeeds(seeds: Uint8Array[]): void {
  if (seeds.length > 16) {
    throw new Error('Maximum of 16 seeds allowed');
  }
  for (let i = 0; i < seeds.length; i++) {
    const seed = seeds[i];
    if (seed && seed.length > 32) {
      throw new Error('Seed ' + i + ' exceeds maximum length of 32 bytes');
    }
  }
}

/**
 * Derives a Program-Derived Address (PDA) from seeds and program ID.
 * 
 * Algorithm:
 * 1. For bump = 255 down to 0:
 *    a. Concatenate: seeds + [bump] + programId + "ProgramDerivedAddress"
 *    b. SHA-256 hash the concatenation
 *    c. If result is off the ed25519 curve, return it
 * 2. If no valid PDA found after 256 attempts, throw error
 * 
 * @param seeds - Array of seed buffers (max 32 bytes each, max 16 seeds)
 * @param programId - The program ID (base58 string)
 * @returns Tuple of [derivedAddress (base58), bumpSeed]
 */
export async function findProgramAddress(
  seeds: Uint8Array[],
  programId: string
): Promise<[string, number]> {
  validateSeeds(seeds);

  const programIdBytes = decodeBase58(programId);
  if (programIdBytes.length !== 32) {
    throw new Error('Program ID must be 32 bytes');
  }

  // Try bump seeds from 255 down to 0
  for (let bump = 255; bump >= 0; bump--) {
    const buffer = buildPdaBuffer(seeds, programIdBytes, bump);
    const hash = await sha256Async(buffer);
    
    if (!isOnCurve(hash)) {
      const result = encodeBase58(hash);
      return [result, bump];
    }
  }

  throw new Error('Unable to find a valid PDA');
}

/**
 * Synchronous version of findProgramAddress.
 * Uses synchronous SHA-256 (Node.js crypto module).
 */
export function findProgramAddressSync(
  seeds: Uint8Array[],
  programId: string
): [string, number] {
  validateSeeds(seeds);

  const programIdBytes = decodeBase58(programId);
  if (programIdBytes.length !== 32) {
    throw new Error('Program ID must be 32 bytes');
  }

  // Try bump seeds from 255 down to 0
  for (let bump = 255; bump >= 0; bump--) {
    const buffer = buildPdaBuffer(seeds, programIdBytes, bump);
    const hash = sha256Sync(buffer);
    
    if (!isOnCurve(hash)) {
      const result = encodeBase58(hash);
      return [result, bump];
    }
  }

  throw new Error('Unable to find a valid PDA');
}

/**
 * Creates a seed buffer from various input types.
 * 
 * @param value - The value to convert to a seed
 * @returns Uint8Array suitable for PDA derivation
 */
export function createSeed(value: string | Uint8Array | bigint | number): Uint8Array {
  if (value instanceof Uint8Array) {
    return value;
  }
  
  if (typeof value === 'string') {
    return new TextEncoder().encode(value);
  }
  
  if (typeof value === 'bigint') {
    // Convert bigint to 8-byte buffer (u64 little-endian)
    const buffer = new Uint8Array(8);
    let n = value;
    for (let i = 0; i < 8; i++) {
      buffer[i] = Number(n & BigInt(0xff));
      n >>= BigInt(8);
    }
    return buffer;
  }
  
  if (typeof value === 'number') {
    // Assume u64
    return createSeed(BigInt(value));
  }
  
  throw new Error('Cannot create seed from value');
}

/**
 * Creates a public key seed from a base58-encoded address.
 * 
 * @param address - Base58-encoded public key
 * @returns 32-byte Uint8Array
 */
export function createPublicKeySeed(address: string): Uint8Array {
  const decoded = decodeBase58(address);
  if (decoded.length !== 32) {
    throw new Error('Invalid public key length: expected 32, got ' + decoded.length);
  }
  return decoded;
}

// Legacy export for backwards compatibility
export { findProgramAddress as derivePda };

/**
 * Borsh-compatible instruction data serializer.
 * 
 * This module handles serializing instruction arguments into the binary format
 * expected by Solana programs using Borsh serialization.
 */

/**
 * Instruction argument schema for serialization.
 */
export interface ArgSchema {
  /** Argument name */
  name: string;
  /** Argument type */
  type: ArgType;
}

/**
 * Supported argument types for Borsh serialization.
 */
export type ArgType =
  | 'u8' | 'u16' | 'u32' | 'u64' | 'u128'
  | 'i8' | 'i16' | 'i32' | 'i64' | 'i128'
  | 'bool'
  | 'string'
  | 'pubkey'
  | { vec: ArgType }
  | { option: ArgType }
  | { array: [ArgType, number] };

/**
 * Serializes instruction arguments into a Buffer using Borsh encoding.
 * 
 * @param discriminator - The 8-byte instruction discriminator
 * @param args - Arguments to serialize
 * @param schema - Schema defining argument types
 * @returns Serialized instruction data
 */
export function serializeInstructionData(
  discriminator: Uint8Array,
  args: Record<string, unknown>,
  schema: ArgSchema[]
): Buffer {
  const buffers: Buffer[] = [Buffer.from(discriminator)];
  
  for (const field of schema) {
    const value = args[field.name];
    const serialized = serializeValue(value, field.type);
    buffers.push(serialized);
  }
  
  return Buffer.concat(buffers);
}

function serializeValue(value: unknown, type: ArgType): Buffer {
  if (typeof type === 'string') {
    return serializePrimitive(value, type);
  }
  
  if ('vec' in type) {
    return serializeVec(value as unknown[], type.vec);
  }
  
  if ('option' in type) {
    return serializeOption(value, type.option);
  }
  
  if ('array' in type) {
    return serializeArray(value as unknown[], type.array[0], type.array[1]);
  }
  
  throw new Error(`Unknown type: ${JSON.stringify(type)}`);
}

function serializePrimitive(value: unknown, type: string): Buffer {
  switch (type) {
    case 'u8':
      return Buffer.from([value as number]);
    case 'u16':
      const u16 = Buffer.alloc(2);
      u16.writeUInt16LE(value as number, 0);
      return u16;
    case 'u32':
      const u32 = Buffer.alloc(4);
      u32.writeUInt32LE(value as number, 0);
      return u32;
    case 'u64':
      const u64 = Buffer.alloc(8);
      u64.writeBigUInt64LE(BigInt(value as string | number | bigint), 0);
      return u64;
    case 'u128':
      // u128 is 16 bytes, little-endian
      const u128 = Buffer.alloc(16);
      const bigU128 = BigInt(value as string | number | bigint);
      u128.writeBigUInt64LE(bigU128 & BigInt('0xFFFFFFFFFFFFFFFF'), 0);
      u128.writeBigUInt64LE(bigU128 >> BigInt(64), 8);
      return u128;
    case 'i8':
      return Buffer.from([value as number]);
    case 'i16':
      const i16 = Buffer.alloc(2);
      i16.writeInt16LE(value as number, 0);
      return i16;
    case 'i32':
      const i32 = Buffer.alloc(4);
      i32.writeInt32LE(value as number, 0);
      return i32;
    case 'i64':
      const i64 = Buffer.alloc(8);
      i64.writeBigInt64LE(BigInt(value as string | number | bigint), 0);
      return i64;
    case 'i128':
      const i128 = Buffer.alloc(16);
      const bigI128 = BigInt(value as string | number | bigint);
      i128.writeBigInt64LE(bigI128 & BigInt('0xFFFFFFFFFFFFFFFF'), 0);
      i128.writeBigInt64LE(bigI128 >> BigInt(64), 8);
      return i128;
    case 'bool':
      return Buffer.from([value as boolean ? 1 : 0]);
    case 'string':
      const str = value as string;
      const strBytes = Buffer.from(str, 'utf-8');
      const strLen = Buffer.alloc(4);
      strLen.writeUInt32LE(strBytes.length, 0);
      return Buffer.concat([strLen, strBytes]);
    case 'pubkey':
      // Public key is 32 bytes
      // In production, decode base58 to 32 bytes
      return Buffer.alloc(32, 0);
    default:
      throw new Error(`Unknown primitive type: ${type}`);
  }
}

function serializeVec(values: unknown[], elementType: ArgType): Buffer {
  const len = Buffer.alloc(4);
  len.writeUInt32LE(values.length, 0);
  
  const elementBuffers = values.map(v => serializeValue(v, elementType));
  return Buffer.concat([len, ...elementBuffers]);
}

function serializeOption(value: unknown, innerType: ArgType): Buffer {
  if (value === null || value === undefined) {
    return Buffer.from([0]); // None
  }
  
  const inner = serializeValue(value, innerType);
  return Buffer.concat([Buffer.from([1]), inner]); // Some
}

function serializeArray(
  values: unknown[],
  elementType: ArgType,
  length: number
): Buffer {
  if (values.length !== length) {
    throw new Error(
      `Array length mismatch: expected ${length}, got ${values.length}`
    );
  }
  
  const elementBuffers = values.map(v => serializeValue(v, elementType));
  return Buffer.concat(elementBuffers);
}

import { inflate } from 'pako';

export type FrameMode = 'state' | 'append' | 'list';
export type FrameOp = 'create' | 'upsert' | 'patch' | 'delete' | 'snapshot';

export interface EntityFrame<T = unknown> {
  mode: FrameMode;
  entity: string;
  op: FrameOp;
  key: string;
  data: T;
  append?: string[];
}

export interface SnapshotEntity<T = unknown> {
  key: string;
  data: T;
}

export interface SnapshotFrame<T = unknown> {
  mode: FrameMode;
  entity: string;
  op: 'snapshot';
  data: SnapshotEntity<T>[];
}

export type Frame<T = unknown> = EntityFrame<T> | SnapshotFrame<T>;

interface CompressedFrame {
  compressed: 'gzip';
  data: string;
}

function isCompressedFrame(obj: unknown): obj is CompressedFrame {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    (obj as CompressedFrame).compressed === 'gzip' &&
    typeof (obj as CompressedFrame).data === 'string'
  );
}

function decompressGzip(base64Data: string): string {
  const binaryString = atob(base64Data);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  const decompressed = inflate(bytes);
  return new TextDecoder().decode(decompressed);
}

function parseAndDecompress(jsonString: string): Frame {
  const parsed = JSON.parse(jsonString);

  if (isCompressedFrame(parsed)) {
    const decompressedJson = decompressGzip(parsed.data);
    const frame = JSON.parse(decompressedJson) as Frame;
    return frame;
  }

  return parsed as Frame;
}

export function isSnapshotFrame<T>(frame: Frame<T>): frame is SnapshotFrame<T> {
  return frame.op === 'snapshot';
}

export function parseFrame(data: ArrayBuffer | string): Frame {
  if (typeof data === 'string') {
    return parseAndDecompress(data);
  }

  const decoder = new TextDecoder('utf-8');
  const jsonString = decoder.decode(data);
  return parseAndDecompress(jsonString);
}

export async function parseFrameFromBlob(blob: Blob): Promise<Frame> {
  const arrayBuffer = await blob.arrayBuffer();
  return parseFrame(arrayBuffer);
}

export function isValidFrame(frame: unknown): frame is Frame {
  if (typeof frame !== 'object' || frame === null) {
    return false;
  }

  const f = frame as Record<string, unknown>;

  if (
    typeof f['entity'] !== 'string' ||
    typeof f['op'] !== 'string' ||
    typeof f['mode'] !== 'string' ||
    !['state', 'append', 'list'].includes(f['mode'] as string)
  ) {
    return false;
  }

  if (f['op'] === 'snapshot') {
    return Array.isArray(f['data']);
  }

  return (
    typeof f['key'] === 'string' &&
    ['create', 'upsert', 'patch', 'delete'].includes(f['op'] as string)
  );
}

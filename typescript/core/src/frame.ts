export type FrameMode = 'state' | 'append' | 'list';
export type FrameOp = 'create' | 'upsert' | 'patch' | 'delete' | 'snapshot';

export interface EntityFrame<T = unknown> {
  mode: FrameMode;
  entity: string;
  op: FrameOp;
  key: string;
  data: T;
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

export function isSnapshotFrame<T>(frame: Frame<T>): frame is SnapshotFrame<T> {
  return frame.op === 'snapshot';
}

export function parseFrame(data: ArrayBuffer | string): EntityFrame {
  if (typeof data === 'string') {
    return JSON.parse(data) as EntityFrame;
  }

  const decoder = new TextDecoder('utf-8');
  const jsonString = decoder.decode(data);
  return JSON.parse(jsonString) as EntityFrame;
}

export async function parseFrameFromBlob(blob: Blob): Promise<EntityFrame> {
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

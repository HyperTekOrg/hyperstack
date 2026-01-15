export type FrameMode = 'state' | 'append' | 'list';
export type FrameOp = 'create' | 'upsert' | 'patch' | 'delete';

export interface EntityFrame<T = unknown> {
  mode: FrameMode;
  entity: string;
  op: FrameOp;
  key: string;
  data: T;
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

export function isValidFrame(frame: unknown): frame is EntityFrame {
  if (typeof frame !== 'object' || frame === null) {
    return false;
  }

  const f = frame as Record<string, unknown>;

  return (
    typeof f['entity'] === 'string' &&
    typeof f['key'] === 'string' &&
    typeof f['op'] === 'string' &&
    ['create', 'upsert', 'patch', 'delete'].includes(f['op'] as string) &&
    typeof f['mode'] === 'string' &&
    ['state', 'append', 'list'].includes(f['mode'] as string)
  );
}

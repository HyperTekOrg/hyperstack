import { describe, it, expect } from 'vitest';
import { parseFrame, isSnapshotFrame } from './frame';
import { deflate } from 'pako';

describe('HyperStack SDK', () => {
  it('should export HyperStack class', async () => {
    const { HyperStack } = await import('./index');
    expect(HyperStack).toBeDefined();
    expect(typeof HyperStack.connect).toBe('function');
  });

  it('should export ConnectionManager', async () => {
    const { ConnectionManager } = await import('./index');
    expect(ConnectionManager).toBeDefined();
  });

  it('should export EntityStore', async () => {
    const { EntityStore } = await import('./index');
    expect(EntityStore).toBeDefined();
  });
});

describe('Frame parsing', () => {
  it('should parse uncompressed entity frames', () => {
    const frame = {
      mode: 'list',
      entity: 'test/list',
      op: 'upsert',
      key: '1',
      data: { id: 1 },
    };
    const result = parseFrame(JSON.stringify(frame));
    expect(result.op).toBe('upsert');
    expect(result.entity).toBe('test/list');
    expect(isSnapshotFrame(result)).toBe(false);
  });

  it('should parse uncompressed snapshot frames', () => {
    const frame = {
      mode: 'list',
      entity: 'test/list',
      op: 'snapshot',
      data: [{ key: '1', data: { id: 1 } }],
    };
    const result = parseFrame(JSON.stringify(frame));
    expect(result.op).toBe('snapshot');
    expect(result.entity).toBe('test/list');
    expect(isSnapshotFrame(result)).toBe(true);
    if (isSnapshotFrame(result)) {
      expect(result.data).toHaveLength(1);
      expect(result.data[0].key).toBe('1');
    }
  });

  it('should decompress gzip-compressed snapshot frames', () => {
    const originalFrame = {
      mode: 'list',
      entity: 'test/list',
      op: 'snapshot',
      data: [
        { key: '1', data: { id: 1, name: 'Test Entity' } },
        { key: '2', data: { id: 2, name: 'Another Entity' } },
      ],
    };

    const jsonString = JSON.stringify(originalFrame);
    const compressed = deflate(new TextEncoder().encode(jsonString));
    const base64 = btoa(String.fromCharCode(...compressed));

    const compressedFrame = JSON.stringify({
      compressed: 'gzip',
      data: base64,
    });

    const result = parseFrame(compressedFrame);
    expect(result.op).toBe('snapshot');
    expect(result.entity).toBe('test/list');
    expect(isSnapshotFrame(result)).toBe(true);
    if (isSnapshotFrame(result)) {
      expect(result.data).toHaveLength(2);
      expect(result.data[0].key).toBe('1');
      expect(result.data[0].data).toEqual({ id: 1, name: 'Test Entity' });
      expect(result.data[1].key).toBe('2');
    }
  });
});

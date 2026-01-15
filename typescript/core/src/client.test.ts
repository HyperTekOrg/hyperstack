import { describe, it, expect } from 'vitest';

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

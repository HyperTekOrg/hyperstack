import { DEFAULT_CONFIG, HyperStreamError } from './types';

describe('types', () => {
  describe('DEFAULT_CONFIG', () => {
    it('has sensible defaults', () => {
      expect(DEFAULT_CONFIG.websocketUrl).toBe('ws://localhost:8080');
      expect(DEFAULT_CONFIG.maxReconnectAttempts).toBe(5);
      expect(DEFAULT_CONFIG.reconnectIntervals).toHaveLength(5);
      expect(DEFAULT_CONFIG.autoSubscribeDefault).toBe(true);
    });
  });

  describe('HyperStreamError', () => {
    it('creates error with code and details', () => {
      const error = new HyperStreamError('test message', 'TEST_CODE', { foo: 'bar' });

      expect(error.message).toBe('test message');
      expect(error.code).toBe('TEST_CODE');
      expect(error.details).toEqual({ foo: 'bar' });
      expect(error.name).toBe('HyperStreamError');
    });
  });
});

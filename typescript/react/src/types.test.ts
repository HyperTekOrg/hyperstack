import { DEFAULT_CONFIG, HyperStackError } from 'hyperstack-typescript';

describe('types', () => {
  describe('DEFAULT_CONFIG', () => {
    it('has sensible defaults', () => {
      expect(DEFAULT_CONFIG.maxReconnectAttempts).toBe(5);
      expect(DEFAULT_CONFIG.reconnectIntervals).toHaveLength(5);
      expect(DEFAULT_CONFIG.maxEntriesPerView).toBe(10_000);
    });
  });

  describe('HyperStackError', () => {
    it('creates error with code and details', () => {
      const error = new HyperStackError('test message', 'TEST_CODE', { foo: 'bar' });

      expect(error.message).toBe('test message');
      expect(error.code).toBe('TEST_CODE');
      expect(error.details).toEqual({ foo: 'bar' });
      expect(error.name).toBe('HyperStackError');
    });
  });
});

import type { Frame } from './frame';
import { parseFrame, parseFrameFromBlob } from './frame';
import type { ConnectionState, Subscription, HyperStackConfig, ConnectionStateCallback } from './types';
import { DEFAULT_CONFIG, HyperStackError } from './types';

export type FrameHandler = <T>(frame: Frame<T>) => void;

export class ConnectionManager {
  private ws: WebSocket | null = null;
  private websocketUrl: string;
  private reconnectIntervals: number[];
  private maxReconnectAttempts: number;
  private reconnectAttempts = 0;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private pingInterval: ReturnType<typeof setInterval> | null = null;
  private currentState: ConnectionState = 'disconnected';
  private subscriptionQueue: Subscription[] = [];
  private activeSubscriptions: Set<string> = new Set();

  private frameHandlers: Set<FrameHandler> = new Set();
  private stateHandlers: Set<ConnectionStateCallback> = new Set();

  constructor(config: HyperStackConfig) {
    if (!config.websocketUrl) {
      throw new HyperStackError('websocketUrl is required', 'INVALID_CONFIG');
    }
    this.websocketUrl = config.websocketUrl;
    this.reconnectIntervals = config.reconnectIntervals ?? DEFAULT_CONFIG.reconnectIntervals;
    this.maxReconnectAttempts = config.maxReconnectAttempts ?? DEFAULT_CONFIG.maxReconnectAttempts;

    if (config.initialSubscriptions) {
      this.subscriptionQueue.push(...config.initialSubscriptions);
    }
  }

  getState(): ConnectionState {
    return this.currentState;
  }

  onFrame(handler: FrameHandler): () => void {
    this.frameHandlers.add(handler);
    return () => {
      this.frameHandlers.delete(handler);
    };
  }

  onStateChange(handler: ConnectionStateCallback): () => void {
    this.stateHandlers.add(handler);
    return () => {
      this.stateHandlers.delete(handler);
    };
  }

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      if (
        this.ws?.readyState === WebSocket.OPEN ||
        this.ws?.readyState === WebSocket.CONNECTING ||
        this.currentState === 'connecting'
      ) {
        resolve();
        return;
      }

      this.updateState('connecting');

      try {
        this.ws = new WebSocket(this.websocketUrl);

        this.ws.onopen = () => {
          this.reconnectAttempts = 0;
          this.updateState('connected');
          this.startPingInterval();
          this.resubscribeActive();
          this.flushSubscriptionQueue();
          resolve();
        };

        this.ws.onmessage = async (event) => {
          try {
            let frame: Frame;

            if (event.data instanceof ArrayBuffer) {
              frame = parseFrame(event.data);
            } else if (event.data instanceof Blob) {
              frame = await parseFrameFromBlob(event.data);
            } else if (typeof event.data === 'string') {
              frame = parseFrame(event.data);
            } else {
              throw new HyperStackError(
                `Unsupported message type: ${typeof event.data}`,
                'PARSE_ERROR'
              );
            }

            this.notifyFrameHandlers(frame);
          } catch (error) {
            console.error('[hyperstack] Error parsing frame:', error);
            this.updateState('error', 'Failed to parse frame from server');
          }
        };

        this.ws.onerror = () => {
          const error = new HyperStackError('WebSocket connection error', 'CONNECTION_ERROR');
          this.updateState('error', error.message);
          if (this.currentState === 'connecting') {
            reject(error);
          }
        };

        this.ws.onclose = () => {
          this.stopPingInterval();
          this.ws = null;

          if (this.currentState !== 'disconnected') {
            this.handleReconnect();
          }
        };
      } catch (error) {
        const hsError = new HyperStackError(
          'Failed to create WebSocket connection',
          'CONNECTION_ERROR',
          error
        );
        this.updateState('error', hsError.message);
        reject(hsError);
      }
    });
  }

  disconnect(): void {
    this.clearReconnectTimeout();
    this.stopPingInterval();
    this.updateState('disconnected');

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  subscribe(subscription: Subscription): void {
    const subKey = this.makeSubKey(subscription);

    if (this.currentState === 'connected' && this.ws?.readyState === WebSocket.OPEN) {
      if (this.activeSubscriptions.has(subKey)) {
        console.log('[hyperstack] Skipping already active subscription:', subKey);
        return;
      }
      const subMsg = { type: 'subscribe', ...subscription };
      console.log('[hyperstack] Sending subscribe:', subKey);
      this.ws.send(JSON.stringify(subMsg));
      this.activeSubscriptions.add(subKey);
    } else {
      const alreadyQueued = this.subscriptionQueue.some(
        (s) => this.makeSubKey(s) === subKey
      );
      if (alreadyQueued) {
        console.log('[hyperstack] Skipping duplicate queue entry:', subKey);
      } else {
        console.log('[hyperstack] Queuing subscription:', subKey, '| Queue:', this.subscriptionQueue.map(s => this.makeSubKey(s)));
        this.subscriptionQueue.push(subscription);
      }
    }
  }

  unsubscribe(view: string, key?: string): void {
    const subscription: Subscription = { view, key };
    const subKey = this.makeSubKey(subscription);
    
    if (this.activeSubscriptions.has(subKey)) {
      this.activeSubscriptions.delete(subKey);
      
      if (this.ws?.readyState === WebSocket.OPEN) {
        const unsubMsg = { type: 'unsubscribe', view, key };
        this.ws.send(JSON.stringify(unsubMsg));
      }
    }
  }

  isConnected(): boolean {
    return this.currentState === 'connected' && this.ws?.readyState === WebSocket.OPEN;
  }

  private makeSubKey(subscription: Subscription): string {
    return `${subscription.view}:${subscription.key ?? '*'}:${subscription.partition ?? ''}`;
  }

  private flushSubscriptionQueue(): void {
    console.log('[hyperstack] Flushing subscription queue:', this.subscriptionQueue.map(s => this.makeSubKey(s)));
    while (this.subscriptionQueue.length > 0) {
      const sub = this.subscriptionQueue.shift();
      if (sub) {
        this.subscribe(sub);
      }
    }
  }

  private resubscribeActive(): void {
    console.log('[hyperstack] Resubscribing active:', Array.from(this.activeSubscriptions));
    for (const subKey of this.activeSubscriptions) {
      const [view, key, partition] = subKey.split(':');
      const subscription: Subscription = {
        view: view ?? '',
        key: key === '*' ? undefined : key,
        partition: partition || undefined,
      };

      if (this.ws?.readyState === WebSocket.OPEN) {
        const subMsg = { type: 'subscribe', ...subscription };
        console.log('[hyperstack] Resubscribe sending:', subKey);
        this.ws.send(JSON.stringify(subMsg));
      }
    }
  }

  private updateState(state: ConnectionState, error?: string): void {
    this.currentState = state;
    for (const handler of this.stateHandlers) {
      handler(state, error);
    }
  }

  private notifyFrameHandlers(frame: Frame): void {
    for (const handler of this.frameHandlers) {
      handler(frame);
    }
  }

  private handleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.updateState(
        'error',
        `Max reconnection attempts (${this.reconnectAttempts}) reached`
      );
      return;
    }

    this.updateState('reconnecting');

    const attemptIndex = Math.min(this.reconnectAttempts, this.reconnectIntervals.length - 1);
    const delay = this.reconnectIntervals[attemptIndex] ?? 1000;

    this.reconnectAttempts++;

    this.reconnectTimeout = setTimeout(() => {
      this.connect().catch(() => {
        /* retry handled by onclose */
      });
    }, delay);
  }

  private clearReconnectTimeout(): void {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  private startPingInterval(): void {
    this.stopPingInterval();
    this.pingInterval = setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send('{"type":"ping"}');
      }
    }, 15000);
  }

  private stopPingInterval(): void {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }
}

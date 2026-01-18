import { ConnectionState, Frame, Subscription, HyperSDKConfig, DEFAULT_CONFIG } from './types';

// Handler types for the ConnectionManager callbacks
export type FrameHandler = <T>(frame: Frame<T>) => void;               // called when Frame arrives from WebSocket
export type StateHandler = (state: ConnectionState, error?: string) => void; // called on connection state changes

// Manages WebSocket connection lifecycle and subscription queuing
export class ConnectionManager {
  private ws: WebSocket | null = null;
  private config: HyperSDKConfig;
  private reconnectAttempts = 0;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private pingInterval: ReturnType<typeof setInterval> | null = null;
  private currentState: ConnectionState = 'disconnected';
  private subscriptionQueue: Subscription[] = [];

  private onFrame?: FrameHandler;
  private onStateChange?: StateHandler;

  constructor(config: Partial<HyperSDKConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  // set callbacks
  setHandlers(handlers: {
    onFrame?: FrameHandler;
    onStateChange?: StateHandler;
  }) {
    this.onFrame = handlers.onFrame;
    this.onStateChange = handlers.onStateChange;
  }

  // public getters for external state inspection
  getState(): ConnectionState {
    return this.currentState;
  }

  getConfig(): HyperSDKConfig {
    return this.config;
  }

  // Initiate WebSocket connection with subscription restoration
  connect(): void {
    // prevent duplicate connections
    if (this.ws?.readyState === WebSocket.OPEN ||
      this.ws?.readyState === WebSocket.CONNECTING ||
      this.currentState === 'connecting') {
      console.log('Connection already exists or in progress, skipping connect');
      return;
    }

    console.log('[Hyperstack] Connecting to WebSocket...');
    this.updateState('connecting'); // update UI state immediately

    try {
      this.ws = new WebSocket(this.config.websocketUrl!);

      this.ws.onopen = () => {
        console.log('[Hyperstack] WebSocket connected');
        this.reconnectAttempts = 0;     // reset retry counter on successful connect
        this.updateState('connected');  // notify store/UI of successful connection
        
        this.startPingInterval();       // start keep-alive ping

        // send global subscriptions first (from config)
        if (this.config.initialSubscriptions) {
          for (const sub of this.config.initialSubscriptions) {
            this.subscribe(sub);
          }
        }

        // flush queued subscriptions from when we were offline
        while (this.subscriptionQueue.length > 0) {
          const sub = this.subscriptionQueue.shift()!;
          this.subscribe(sub);
        }
      };

      this.ws.onmessage = async (event) => {
        try {
          let frame: Frame;

          if (event.data instanceof ArrayBuffer) {
            frame = this.parseBinaryFrame(event.data);
          } else if (event.data instanceof Blob) {
            const arrayBuffer = await event.data.arrayBuffer();
            frame = this.parseBinaryFrame(arrayBuffer);
          } else if (typeof event.data === 'string') {
            frame = JSON.parse(event.data) as Frame;
          } else {
            throw new Error(`Unsupported message type: ${typeof event.data}`);
          }

          this.onFrame?.(frame);
        } catch (error) {
          console.error('Failed to parse frame:', error);
          this.updateState('error', 'Failed to parse frame from server');
        }
      };

      // WebSocket error handling
      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        this.updateState('error', 'WebSocket connection error'); // Notify store/UI of errors
      };

      // connection lost handler with auto-reconnection
      this.ws.onclose = () => {
        console.log('WebSocket disconnected');
        this.stopPingInterval();
        this.ws = null;

        // only auto-reconnect if we didn't explicitly disconnect
        // this preserves subscriptions across temporary network issues
        if (this.currentState !== 'disconnected') {
          this.handleReconnect();
        }
      };

    } catch (error) {
      console.error('Failed to create WebSocket:', error);
      this.updateState('error', 'Failed to create WebSocket connection');
    }
  }

  disconnect(): void {
    this.clearReconnectTimeout();
    this.stopPingInterval();

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.updateState('disconnected');
  }

  updateConfig(newConfig: Partial<HyperSDKConfig>): void {
    this.config = { ...this.config, ...newConfig };
  }

  subscribe(subscription: Subscription): void {
    if (this.currentState === 'connected' && this.ws && this.ws.readyState === WebSocket.OPEN) {
      console.log('[Hyperstack] Subscribing to:', subscription.view);
      const subMsg = { type: 'subscribe', ...subscription };
      this.ws.send(JSON.stringify(subMsg));
    } else {
      this.subscriptionQueue.push(subscription);
    }
  }

  unsubscribe(view: string, key?: string): void {
    if (this.currentState === 'connected' && this.ws?.readyState === WebSocket.OPEN) {
      const unsubMsg = { type: 'unsubscribe', view, key };
      this.ws.send(JSON.stringify(unsubMsg));
    }
  }

  private parseBinaryFrame(data: ArrayBuffer): Frame {
    const decoder = new TextDecoder('utf-8');
    const jsonString = decoder.decode(data);
    return JSON.parse(jsonString) as Frame;
  }

  // Internal state change handler - notifies store and triggers UI re-renders
  private updateState(state: ConnectionState, error?: string): void {
    this.currentState = state;
    this.onStateChange?.(state, error);
  }

  // Auto-reconnection with true exponential backoff protection
  private handleReconnect(): void {
    const intervals = this.config.reconnectIntervals || [1000, 2000, 4000, 8000, 16000];
    const maxAttempts = this.config.maxReconnectAttempts || intervals.length;

    if (this.reconnectAttempts >= maxAttempts) {
      // give up after max attempts to avoid infinite retry loops
      this.updateState('error', `Max reconnection attempts (${this.reconnectAttempts}) reached`);
      return;
    }

    this.updateState('reconnecting'); // update store/UI to show reconnection status

    // get delay for current attempt (use last interval if we exceed array length)
    const attemptIndex = Math.min(this.reconnectAttempts, intervals.length - 1);
    const delay = intervals[attemptIndex];

    this.reconnectAttempts++;

    // delayed reconnection with exponential backoff
    this.reconnectTimeout = setTimeout(() => {
      console.log(`Reconnect attempt ${this.reconnectAttempts} after ${delay}ms delay`);
      this.connect(); // recursive call - will restore subscriptions on success
    }, delay);
  }

  // Cleanup helper for reconnection timer
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

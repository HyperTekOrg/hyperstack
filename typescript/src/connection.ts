import { ConnectionState, EntityFrame, Subscription, HyperSDKConfig, DEFAULT_CONFIG } from './types';

// Handler types for the ConnectionManager callbacks
export type FrameHandler = <T>(frame: EntityFrame<T>) => void;               // called when EntityFrame arrives from WebSocket
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

      // core message handler - parses incoming data into EntityFrames
      this.ws.onmessage = async (event) => {
        try {
          let frame: EntityFrame;

          if (event.data instanceof ArrayBuffer) {
            // binary data as ArrayBuffer
            frame = this.parseBinaryFrame(event.data);
          } else if (event.data instanceof Blob) {
            // binary data as Blob - convert to ArrayBuffer
            const arrayBuffer = await event.data.arrayBuffer();
            frame = this.parseBinaryFrame(arrayBuffer);
          } else if (typeof event.data === 'string') {
            // JSON text data
            frame = JSON.parse(event.data) as EntityFrame;
          } else {
            throw new Error(`Unsupported message type: ${typeof event.data}`);
          }

          // fw parsed frame to store for entity updates
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
      const message = JSON.stringify(subscription);
      this.ws.send(message);
    } else {
      this.subscriptionQueue.push(subscription);
    }
  }

  // Unsubscribe support (feature-gated by server capabilities)
  unsubscribe(_view: string, _key?: string): void {
    if (this.config.supportsUnsubscribe && this.currentState === 'connected' && this.ws) {
      // only send unsubscribe if server supports it - TO DO
      console.warn('Unsubscribe not yet implemented on server side');
    }
  }

  // binary frame parser - converts WebSocket binary data to EntityFrame
  private parseBinaryFrame(data: ArrayBuffer): EntityFrame {
    // server sends JSON Frame serialized as binary bytes
    // convert binary data back to JSON string and parse
    const decoder = new TextDecoder('utf-8');
    const jsonString = decoder.decode(data);

    // parse the Frame JSON sent by projector
    const frame = JSON.parse(jsonString);

    // convert to EntityFrame format expected by SDK
    return {
      mode: frame.mode,
      entity: frame.entity,  // Backend serializes view path as 'entity' field
      op: frame.op,
      key: frame.key,
      data: frame.data
    };
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
        this.ws.send(JSON.stringify({ type: 'ping' }));
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

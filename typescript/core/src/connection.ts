import type { Frame } from './frame';
import { parseFrame, parseFrameFromBlob } from './frame';
import type { ConnectionState, Subscription, HyperStackConfig, ConnectionStateCallback, AuthConfig } from './types';
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

  // Auth-related fields
  private authConfig?: AuthConfig;
  private currentToken?: string;
  private tokenExpiry?: number;

  constructor(config: HyperStackConfig) {
    if (!config.websocketUrl) {
      throw new HyperStackError('websocketUrl is required', 'INVALID_CONFIG');
    }
    this.websocketUrl = config.websocketUrl;
    this.reconnectIntervals = config.reconnectIntervals ?? DEFAULT_CONFIG.reconnectIntervals;
    this.maxReconnectAttempts = config.maxReconnectAttempts ?? DEFAULT_CONFIG.maxReconnectAttempts;
    this.authConfig = config.auth;

    if (config.initialSubscriptions) {
      this.subscriptionQueue.push(...config.initialSubscriptions);
    }
  }

  /**
   * Get or refresh the authentication token
   */
  private async getOrRefreshToken(): Promise<string | undefined> {
    // Return cached token if still valid
    if (this.currentToken && !this.isTokenExpired()) {
      return this.currentToken;
    }

    if (!this.authConfig) {
      return undefined;
    }

    // Option 1: Static token
    if (this.authConfig.token) {
      this.currentToken = this.authConfig.token;
      return this.currentToken;
    }

    // Option 2: Custom token provider
    if (this.authConfig.getToken) {
      try {
        this.currentToken = await this.authConfig.getToken();
        return this.currentToken;
      } catch (error) {
        throw new HyperStackError(
          'Failed to get authentication token',
          'AUTH_REQUIRED',
          error
        );
      }
    }

    // Option 3: Token endpoint (Hyperstack Cloud)
    if (this.authConfig.tokenEndpoint && this.authConfig.publishableKey) {
      try {
        this.currentToken = await this.fetchTokenFromEndpoint();
        return this.currentToken;
      } catch (error) {
        throw new HyperStackError(
          'Failed to fetch authentication token from endpoint',
          'AUTH_REQUIRED',
          error
        );
      }
    }

    return undefined;
  }

  /**
   * Fetch token from token endpoint
   */
  private async fetchTokenFromEndpoint(): Promise<string> {
    if (!this.authConfig?.tokenEndpoint) {
      throw new Error('Token endpoint not configured');
    }

    const response = await fetch(this.authConfig.tokenEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.authConfig.publishableKey || ''}`,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new HyperStackError(
        `Token endpoint returned ${response.status}: ${errorText}`,
        'AUTH_REQUIRED'
      );
    }

    const data = await response.json() as { token: string; expires_at?: number };
    
    if (!data.token) {
      throw new HyperStackError(
        'Token endpoint did not return a token',
        'AUTH_REQUIRED'
      );
    }

    this.tokenExpiry = data.expires_at;
    return data.token;
  }

  /**
   * Check if the current token is expired (or about to expire)
   */
  private isTokenExpired(): boolean {
    if (!this.tokenExpiry) return false;
    // Consider token expired 60 seconds before actual expiry to allow for clock skew
    const bufferSeconds = 60;
    return Date.now() >= (this.tokenExpiry - bufferSeconds) * 1000;
  }

  /**
   * Build WebSocket URL with authentication token
   */
  private buildAuthUrl(token: string | undefined): string {
    if (!token) {
      return this.websocketUrl;
    }

    const separator = this.websocketUrl.includes('?') ? '&' : '?';
    return `${this.websocketUrl}${separator}hs_token=${encodeURIComponent(token)}`;
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

  async connect(): Promise<void> {
    if (
      this.ws?.readyState === WebSocket.OPEN ||
      this.ws?.readyState === WebSocket.CONNECTING ||
      this.currentState === 'connecting'
    ) {
      return;
    }

    this.updateState('connecting');

    // Get fresh token before connecting
    let token: string | undefined;
    try {
      token = await this.getOrRefreshToken();
    } catch (error) {
      this.updateState('error', error instanceof Error ? error.message : 'Failed to get token');
      throw error;
    }

    const wsUrl = this.buildAuthUrl(token);

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(wsUrl);

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
        return;
      }
      const subMsg = { type: 'subscribe', ...subscription };
      this.ws.send(JSON.stringify(subMsg));
      this.activeSubscriptions.add(subKey);
    } else {
      const alreadyQueued = this.subscriptionQueue.some(
        (s) => this.makeSubKey(s) === subKey
      );
      if (!alreadyQueued) {
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
    while (this.subscriptionQueue.length > 0) {
      const sub = this.subscriptionQueue.shift();
      if (sub) {
        this.subscribe(sub);
      }
    }
  }

  private resubscribeActive(): void {
    for (const subKey of this.activeSubscriptions) {
      const [view, key, partition] = subKey.split(':');
      const subscription: Subscription = {
        view: view ?? '',
        key: key === '*' ? undefined : key,
        partition: partition || undefined,
      };

      if (this.ws?.readyState === WebSocket.OPEN) {
        const subMsg = { type: 'subscribe', ...subscription };
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

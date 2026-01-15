import type {
  ConnectionState,
  StackDefinition,
  HyperStackOptions,
  TypedViews,
  ConnectionStateCallback,
  UnsubscribeFn,
} from './types';
import { HyperStackError } from './types';
import { ConnectionManager } from './connection';
import { EntityStore } from './store';
import { SubscriptionRegistry } from './subscription';
import { createTypedViews } from './views';
import type { EntityFrame } from './frame';

export class HyperStack<TStack extends StackDefinition> {
  private readonly connection: ConnectionManager;
  private readonly store: EntityStore;
  private readonly subscriptionRegistry: SubscriptionRegistry;
  private readonly _views: TypedViews<TStack['views']>;
  private readonly stack: TStack;

  private constructor(
    url: string,
    options: HyperStackOptions<TStack>
  ) {
    this.stack = options.stack;
    this.store = new EntityStore();
    this.connection = new ConnectionManager({
      websocketUrl: url,
      reconnectIntervals: options.reconnectIntervals,
      maxReconnectAttempts: options.maxReconnectAttempts,
    });
    this.subscriptionRegistry = new SubscriptionRegistry(this.connection);

    this.connection.onFrame((frame: EntityFrame) => {
      this.store.handleFrame(frame);
    });

    this._views = createTypedViews(this.stack, this.store, this.subscriptionRegistry);
  }

  static async connect<T extends StackDefinition>(
    url: string,
    options: HyperStackOptions<T>
  ): Promise<HyperStack<T>> {
    if (!url) {
      throw new HyperStackError('URL is required', 'INVALID_CONFIG');
    }
    if (!options.stack) {
      throw new HyperStackError('Stack definition is required', 'INVALID_CONFIG');
    }

    const client = new HyperStack(url, options);

    if (options.autoReconnect !== false) {
      await client.connection.connect();
    }

    return client;
  }

  get views(): TypedViews<TStack['views']> {
    return this._views;
  }

  get connectionState(): ConnectionState {
    return this.connection.getState();
  }

  get stackName(): string {
    return this.stack.name;
  }

  onConnectionStateChange(callback: ConnectionStateCallback): UnsubscribeFn {
    return this.connection.onStateChange(callback);
  }

  onFrame(callback: (frame: EntityFrame) => void): UnsubscribeFn {
    return this.connection.onFrame(callback);
  }

  async connect(): Promise<void> {
    await this.connection.connect();
  }

  disconnect(): void {
    this.subscriptionRegistry.clear();
    this.connection.disconnect();
  }

  isConnected(): boolean {
    return this.connection.isConnected();
  }

  clearStore(): void {
    this.store.clear();
  }

  getStore(): EntityStore {
    return this.store;
  }

  getConnection(): ConnectionManager {
    return this.connection;
  }

  getSubscriptionRegistry(): SubscriptionRegistry {
    return this.subscriptionRegistry;
  }
}

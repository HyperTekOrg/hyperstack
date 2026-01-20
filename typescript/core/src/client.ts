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
import { FrameProcessor } from './frame-processor';
import { MemoryAdapter } from './storage/memory-adapter';
import type { StorageAdapter } from './storage/adapter';
import { SubscriptionRegistry } from './subscription';
import { createTypedViews } from './views';
import type { Frame } from './frame';

export interface HyperStackOptionsWithStorage<TStack extends StackDefinition> extends HyperStackOptions<TStack> {
  storage?: StorageAdapter;
  maxEntriesPerView?: number | null;
}

export class HyperStack<TStack extends StackDefinition> {
  private readonly connection: ConnectionManager;
  private readonly storage: StorageAdapter;
  private readonly processor: FrameProcessor;
  private readonly subscriptionRegistry: SubscriptionRegistry;
  private readonly _views: TypedViews<TStack['views']>;
  private readonly stack: TStack;

  private constructor(
    url: string,
    options: HyperStackOptionsWithStorage<TStack>
  ) {
    this.stack = options.stack;
    this.storage = options.storage ?? new MemoryAdapter();
    this.processor = new FrameProcessor(this.storage, {
      maxEntriesPerView: options.maxEntriesPerView,
    });
    this.connection = new ConnectionManager({
      websocketUrl: url,
      reconnectIntervals: options.reconnectIntervals,
      maxReconnectAttempts: options.maxReconnectAttempts,
    });
    this.subscriptionRegistry = new SubscriptionRegistry(this.connection);

    this.connection.onFrame((frame: Frame) => {
      this.processor.handleFrame(frame);
    });

    this._views = createTypedViews(this.stack, this.storage, this.subscriptionRegistry);
  }

  static async connect<T extends StackDefinition>(
    url: string,
    options: HyperStackOptionsWithStorage<T>
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

  get store(): StorageAdapter {
    return this.storage;
  }

  onConnectionStateChange(callback: ConnectionStateCallback): UnsubscribeFn {
    return this.connection.onStateChange(callback);
  }

  onFrame(callback: (frame: Frame) => void): UnsubscribeFn {
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
    this.storage.clear();
  }

  getStore(): StorageAdapter {
    return this.storage;
  }

  getConnection(): ConnectionManager {
    return this.connection;
  }

  getSubscriptionRegistry(): SubscriptionRegistry {
    return this.subscriptionRegistry;
  }
}

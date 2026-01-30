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
import type { WalletAdapter } from './wallet/types';
import type { InstructionDefinition, ExecuteOptions, ExecutionResult } from './instructions';
import { executeInstruction } from './instructions';

export interface HyperStackOptionsWithStorage<TStack extends StackDefinition> extends HyperStackOptions<TStack> {
  storage?: StorageAdapter;
  maxEntriesPerView?: number | null;
}

export interface InstructionExecutorOptions extends Omit<ExecuteOptions, 'wallet'> {
  wallet: WalletAdapter;
}

export type InstructionExecutor = (
  args: Record<string, unknown>,
  options: InstructionExecutorOptions
) => Promise<ExecutionResult>;

export type InstructionsInterface<TInstructions extends Record<string, InstructionDefinition> | undefined> = 
  TInstructions extends Record<string, InstructionDefinition>
    ? { [K in keyof TInstructions]: InstructionExecutor }
    : {};

export class HyperStack<TStack extends StackDefinition> {
  private readonly connection: ConnectionManager;
  private readonly storage: StorageAdapter;
  private readonly processor: FrameProcessor;
  private readonly subscriptionRegistry: SubscriptionRegistry;
  private readonly _views: TypedViews<TStack['views']>;
  private readonly stack: TStack;
  private readonly _instructions: InstructionsInterface<TStack['instructions']>;

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
    this._instructions = this.buildInstructions();
  }

  private buildInstructions(): InstructionsInterface<TStack['instructions']> {
    const instructions = {} as Record<string, InstructionExecutor>;
    
    if (this.stack.instructions) {
      for (const [name, definition] of Object.entries(this.stack.instructions)) {
        instructions[name] = (args: Record<string, unknown>, options: InstructionExecutorOptions) => {
          return executeInstruction(definition as InstructionDefinition, args, options);
        };
      }
    }
    
    return instructions as InstructionsInterface<TStack['instructions']>;
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

  get instructions(): InstructionsInterface<TStack['instructions']> {
    return this._instructions;
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

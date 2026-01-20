export { HyperStack } from './client';
export type { HyperStackOptionsWithStorage } from './client';
export { ConnectionManager } from './connection';
export { SubscriptionRegistry } from './subscription';

export { FrameProcessor } from './frame-processor';
export type { FrameProcessorConfig } from './frame-processor';

export type { StorageAdapter, UpdateCallback, RichUpdateCallback, StorageAdapterConfig } from './storage/adapter';
export { MemoryAdapter } from './storage/memory-adapter';

export { parseFrame, parseFrameFromBlob, isValidFrame, isSnapshotFrame } from './frame';
export type { EntityFrame, SnapshotFrame, SnapshotEntity, Frame, FrameMode, FrameOp } from './frame';

export { createUpdateStream, createRichUpdateStream } from './stream';
export {
  createTypedStateView,
  createTypedListView,
  createTypedViews,
} from './views';

export type {
  ConnectionState,
  Update,
  RichUpdate,
  ViewDef,
  StackDefinition,
  ViewGroup,
  Subscription,
  HyperStackOptions,
  HyperStackConfig,
  TypedViews,
  TypedViewGroup,
  TypedStateView,
  TypedListView,
  SubscribeCallback,
  UnsubscribeFn,
  ConnectionStateCallback,
} from './types';

export { DEFAULT_CONFIG, DEFAULT_MAX_ENTRIES_PER_VIEW, HyperStackError } from './types';

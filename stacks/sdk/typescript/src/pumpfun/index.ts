export interface PumpfunTokenEvents {
  buys?: EventWrapper<Buy>[] | null;
  buys_exact_sol?: any[] | null;
  create?: Create | null;
  create_v2?: Record<string, any> | null;
  sells?: EventWrapper<Sell>[] | null;
}

export interface PumpfunTokenId {
  bonding_curve?: string | null;
  mint?: string | null;
}

export interface PumpfunTokenInfo {
  is_complete?: boolean | null;
  name?: string | null;
  symbol?: string | null;
  uri?: string | null;
}

export interface PumpfunTokenReserves {
  current_price_sol?: number | null;
  market_cap_sol?: number | null;
  real_sol_reserves?: number | null;
  real_token_reserves?: number | null;
  token_total_supply?: number | null;
  virtual_sol_reserves?: number | null;
  virtual_token_reserves?: number | null;
}

export interface PumpfunTokenTrading {
  average_trade_size?: number | null;
  buy_count?: number | null;
  largest_trade?: number | null;
  last_trade_price?: number | null;
  last_trade_timestamp?: number | null;
  last_whale_address?: string | null;
  sell_count?: number | null;
  smallest_trade?: number | null;
  total_buy_exact_sol_volume?: number | null;
  total_buy_volume?: number | null;
  total_sell_volume?: number | null;
  total_trades?: number | null;
  total_volume?: number | null;
  unique_traders?: number | null;
  whale_trade_count?: number | null;
}

export interface PumpfunToken {
  events?: PumpfunTokenEvents;
  id?: PumpfunTokenId;
  info?: PumpfunTokenInfo;
  reserves?: PumpfunTokenReserves;
  trading?: PumpfunTokenTrading;
  bonding_curve_snapshot?: BondingCurve | null;
}

export interface Create {
  mint?: string;
  mint_authority?: string;
  bonding_curve?: string;
  associated_bonding_curve?: string;
  global?: string;
  mpl_token_metadata?: string;
  metadata?: string;
  user?: string;
  system_program?: string;
  token_program?: string;
  associated_token_program?: string;
  rent?: string;
  event_authority?: string;
  program?: string;
  name?: string;
  symbol?: string;
  uri?: string;
  creator?: string;
}

export interface Buy {
  global?: string;
  fee_recipient?: string;
  mint?: string;
  bonding_curve?: string;
  associated_bonding_curve?: string;
  associated_user?: string;
  user?: string;
  system_program?: string;
  token_program?: string;
  creator_vault?: string;
  event_authority?: string;
  program?: string;
  global_volume_accumulator?: string;
  user_volume_accumulator?: string;
  fee_config?: string;
  fee_program?: string;
  amount?: number;
  max_sol_cost?: number;
  track_volume?: Record<string, any>;
}

export interface Sell {
  global?: string;
  fee_recipient?: string;
  mint?: string;
  bonding_curve?: string;
  associated_bonding_curve?: string;
  associated_user?: string;
  user?: string;
  system_program?: string;
  creator_vault?: string;
  token_program?: string;
  event_authority?: string;
  program?: string;
  fee_config?: string;
  fee_program?: string;
  amount?: number;
  min_sol_output?: number;
}

export interface BondingCurve {
  virtual_token_reserves?: number;
  virtual_sol_reserves?: number;
  real_token_reserves?: number;
  real_sol_reserves?: number;
  token_total_supply?: number;
  complete?: boolean;
  creator?: string;
  is_mayhem_mode?: boolean;
}

export interface BuysEvent {
  amount: number;
  max_sol_cost: number;
}

export interface BuysExactSolEvent {
  spendable_sol_in: number;
  min_tokens_out: number;
}

export interface CreateEvent {}

export interface CreateV2Event {}

export interface SellsEvent {}

export type ConfigStatus = "Paused" | "Active";

/**
 * Wrapper for event data that includes context metadata.
 * Events are automatically wrapped in this structure at runtime.
 */
export interface EventWrapper<T> {
  /** Unix timestamp when the event was processed */
  timestamp: number;
  /** The event-specific data */
  data: T;
  /** Optional blockchain slot number */
  slot?: number;
  /** Optional transaction signature */
  signature?: string;
}

// ============================================================================
// View Definition Types (framework-agnostic)
// ============================================================================

/** View definition with embedded entity type */
export interface ViewDef<T, TMode extends 'state' | 'list'> {
  readonly mode: TMode;
  readonly view: string;
  /** Phantom field for type inference - not present at runtime */
  readonly _entity?: T;
}

/** Helper to create typed state view definitions (keyed lookups) */
function stateView<T>(view: string): ViewDef<T, 'state'> {
  return { mode: 'state', view } as const;
}

/** Helper to create typed list view definitions (collections) */
function listView<T>(view: string): ViewDef<T, 'list'> {
  return { mode: 'list', view } as const;
}

// ============================================================================
// Stack Definition
// ============================================================================

/** Stack definition for PumpfunStream with 1 entities */
export const PUMPFUN_STREAM_STACK = {
  name: 'pumpfun-stream',
  url: 'wss://pumpfun.stack.usehyperstack.com',
  views: {
    PumpfunToken: {
      state: stateView<PumpfunToken>('PumpfunToken/state'),
      list: listView<PumpfunToken>('PumpfunToken/list'),
    },
  },
} as const;

/** Type alias for the stack */
export type PumpfunStreamStack = typeof PUMPFUN_STREAM_STACK;

/** Entity types in this stack */
export type PumpfunStreamEntity = PumpfunToken;

/** Default export for convenience */
export default PUMPFUN_STREAM_STACK;
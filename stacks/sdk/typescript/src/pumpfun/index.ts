import { z } from 'zod';
import { pda, literal, account, arg, bytes } from 'hyperstack-typescript';

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
  resolved_image?: string | null;
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

export const EventWrapperSchema = <T extends z.ZodTypeAny>(data: T) => z.object({
  timestamp: z.number(),
  data,
  slot: z.number().optional(),
  signature: z.string().optional(),
});

export const CreateSchema = z.object({
  mint: z.string().optional(),
  mint_authority: z.string().optional(),
  bonding_curve: z.string().optional(),
  associated_bonding_curve: z.string().optional(),
  global: z.string().optional(),
  mpl_token_metadata: z.string().optional(),
  metadata: z.string().optional(),
  user: z.string().optional(),
  system_program: z.string().optional(),
  token_program: z.string().optional(),
  associated_token_program: z.string().optional(),
  rent: z.string().optional(),
  event_authority: z.string().optional(),
  program: z.string().optional(),
  name: z.string().optional(),
  symbol: z.string().optional(),
  uri: z.string().optional(),
  creator: z.string().optional(),
});

export const BuySchema = z.object({
  global: z.string().optional(),
  fee_recipient: z.string().optional(),
  mint: z.string().optional(),
  bonding_curve: z.string().optional(),
  associated_bonding_curve: z.string().optional(),
  associated_user: z.string().optional(),
  user: z.string().optional(),
  system_program: z.string().optional(),
  token_program: z.string().optional(),
  creator_vault: z.string().optional(),
  event_authority: z.string().optional(),
  program: z.string().optional(),
  global_volume_accumulator: z.string().optional(),
  user_volume_accumulator: z.string().optional(),
  fee_config: z.string().optional(),
  fee_program: z.string().optional(),
  amount: z.number().optional(),
  max_sol_cost: z.number().optional(),
  track_volume: z.record(z.any()).optional(),
});

export const SellSchema = z.object({
  global: z.string().optional(),
  fee_recipient: z.string().optional(),
  mint: z.string().optional(),
  bonding_curve: z.string().optional(),
  associated_bonding_curve: z.string().optional(),
  associated_user: z.string().optional(),
  user: z.string().optional(),
  system_program: z.string().optional(),
  creator_vault: z.string().optional(),
  token_program: z.string().optional(),
  event_authority: z.string().optional(),
  program: z.string().optional(),
  fee_config: z.string().optional(),
  fee_program: z.string().optional(),
  amount: z.number().optional(),
  min_sol_output: z.number().optional(),
});

export const BondingCurveSchema = z.object({
  virtual_token_reserves: z.number().optional(),
  virtual_sol_reserves: z.number().optional(),
  real_token_reserves: z.number().optional(),
  real_sol_reserves: z.number().optional(),
  token_total_supply: z.number().optional(),
  complete: z.boolean().optional(),
  creator: z.string().optional(),
  is_mayhem_mode: z.boolean().optional(),
});

export const BuysEventSchema = z.object({
  amount: z.number(),
  max_sol_cost: z.number(),
});

export const BuysExactSolEventSchema = z.object({
  spendable_sol_in: z.number(),
  min_tokens_out: z.number(),
});

export const CreateEventSchema = z.object({});

export const CreateV2EventSchema = z.object({});

export const SellsEventSchema = z.object({});

export const ConfigStatusSchema = z.enum(["Paused", "Active"]);

export const PumpfunTokenEventsSchema = z.object({
  buys: z.array(EventWrapperSchema(BuySchema)).nullable().optional(),
  buys_exact_sol: z.array(z.any()).nullable().optional(),
  create: CreateSchema.nullable().optional(),
  create_v2: z.record(z.any()).nullable().optional(),
  sells: z.array(EventWrapperSchema(SellSchema)).nullable().optional(),
});

export const PumpfunTokenIdSchema = z.object({
  bonding_curve: z.string().nullable().optional(),
  mint: z.string().nullable().optional(),
});

export const PumpfunTokenInfoSchema = z.object({
  is_complete: z.boolean().nullable().optional(),
  name: z.string().nullable().optional(),
  resolved_image: z.string().nullable().optional(),
  symbol: z.string().nullable().optional(),
  uri: z.string().nullable().optional(),
});

export const PumpfunTokenReservesSchema = z.object({
  current_price_sol: z.number().nullable().optional(),
  market_cap_sol: z.number().nullable().optional(),
  real_sol_reserves: z.number().nullable().optional(),
  real_token_reserves: z.number().nullable().optional(),
  token_total_supply: z.number().nullable().optional(),
  virtual_sol_reserves: z.number().nullable().optional(),
  virtual_token_reserves: z.number().nullable().optional(),
});

export const PumpfunTokenTradingSchema = z.object({
  average_trade_size: z.number().nullable().optional(),
  buy_count: z.number().nullable().optional(),
  largest_trade: z.number().nullable().optional(),
  last_trade_price: z.number().nullable().optional(),
  last_trade_timestamp: z.number().nullable().optional(),
  last_whale_address: z.string().nullable().optional(),
  sell_count: z.number().nullable().optional(),
  smallest_trade: z.number().nullable().optional(),
  total_buy_exact_sol_volume: z.number().nullable().optional(),
  total_buy_volume: z.number().nullable().optional(),
  total_sell_volume: z.number().nullable().optional(),
  total_trades: z.number().nullable().optional(),
  total_volume: z.number().nullable().optional(),
  unique_traders: z.number().nullable().optional(),
  whale_trade_count: z.number().nullable().optional(),
});

export const PumpfunTokenSchema = z.object({
  events: PumpfunTokenEventsSchema.optional(),
  id: PumpfunTokenIdSchema.optional(),
  info: PumpfunTokenInfoSchema.optional(),
  reserves: PumpfunTokenReservesSchema.optional(),
  trading: PumpfunTokenTradingSchema.optional(),
  bonding_curve_snapshot: BondingCurveSchema.nullable().optional(),
});

export const PumpfunTokenCompletedSchema = z.object({
  events: PumpfunTokenEventsSchema,
  id: PumpfunTokenIdSchema,
  info: PumpfunTokenInfoSchema,
  reserves: PumpfunTokenReservesSchema,
  trading: PumpfunTokenTradingSchema,
  bonding_curve_snapshot: BondingCurveSchema,
});

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
  schemas: {
    BondingCurve: BondingCurveSchema,
    Buy: BuySchema,
    BuysEvent: BuysEventSchema,
    BuysExactSolEvent: BuysExactSolEventSchema,
    ConfigStatus: ConfigStatusSchema,
    CreateEvent: CreateEventSchema,
    Create: CreateSchema,
    CreateV2Event: CreateV2EventSchema,
    EventWrapper: EventWrapperSchema,
    PumpfunTokenCompleted: PumpfunTokenCompletedSchema,
    PumpfunTokenEvents: PumpfunTokenEventsSchema,
    PumpfunTokenId: PumpfunTokenIdSchema,
    PumpfunTokenInfo: PumpfunTokenInfoSchema,
    PumpfunTokenReserves: PumpfunTokenReservesSchema,
    PumpfunToken: PumpfunTokenSchema,
    PumpfunTokenTrading: PumpfunTokenTradingSchema,
    Sell: SellSchema,
    SellsEvent: SellsEventSchema,
  },
  pdas: {
    pump: {
      amm_global_config: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('global_config')),
      associated_bonding_curve: pda('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL', account('bonding_curve'), account('token_program'), account('mint')),
      bonding_curve: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('bonding-curve'), account('mint')),
      creator_vault: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('creator-vault'), account('bonding_curve.creator')),
      event_authority: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('__event_authority')),
      fee_config: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('fee_config'), bytes(new Uint8Array([1, 86, 224, 246, 147, 102, 90, 207, 68, 219, 21, 104, 191, 23, 91, 170, 81, 137, 203, 151, 245, 210, 255, 59, 101, 93, 43, 182, 253, 109, 24, 176]))),
      global: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('global')),
      global_incentive_token_account: pda('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL', account('global_volume_accumulator'), account('token_program'), account('mint')),
      global_params: pda('MAyhSmzXzV1pTf7LsNkrNwkWKTo4ougAJ1PPg47MD4e', literal('global-params')),
      global_volume_accumulator: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('global_volume_accumulator')),
      lp_mint: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('pool_lp_mint'), account('pool')),
      mayhem_state: pda('MAyhSmzXzV1pTf7LsNkrNwkWKTo4ougAJ1PPg47MD4e', literal('mayhem-state'), account('mint')),
      mayhem_token_vault: pda('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL', account('sol_vault_authority'), account('token_program'), account('mint')),
      metadata: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('metadata'), bytes(new Uint8Array([11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108, 115, 26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70])), account('mint')),
      mint_authority: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('mint-authority')),
      pool: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('pool'), literal('  '), account('pool_authority'), account('mint'), account('wsol_mint')),
      pool_authority: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('pool-authority'), account('mint')),
      pool_authority_mint_account: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', account('pool_authority'), account('mint'), account('mint')),
      pool_authority_wsol_account: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', account('pool_authority'), account('token_program'), account('wsol_mint')),
      pool_base_token_account: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', account('pool'), account('mint'), account('mint')),
      pool_quote_token_account: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', account('pool'), account('token_program'), account('wsol_mint')),
      program_signer: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', ),
      pump_amm_event_authority: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('__event_authority')),
      sharing_config: pda('pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ', literal('sharing-config'), account('mint')),
      sol_vault: pda('MAyhSmzXzV1pTf7LsNkrNwkWKTo4ougAJ1PPg47MD4e', literal('sol-vault')),
      sol_vault_authority: pda('MAyhSmzXzV1pTf7LsNkrNwkWKTo4ougAJ1PPg47MD4e', literal('sol-vault')),
      user_ata: pda('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL', account('user'), account('token_program'), account('mint')),
      user_pool_token_account: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', account('pool_authority'), account('token_2022_program'), account('lp_mint')),
      user_volume_accumulator: pda('6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P', literal('user_volume_accumulator'), account('user')),
    },
  },
} as const;

/** Type alias for the stack */
export type PumpfunStreamStack = typeof PUMPFUN_STREAM_STACK;

/** Entity types in this stack */
export type PumpfunStreamEntity = PumpfunToken;

/** Default export for convenience */
export default PUMPFUN_STREAM_STACK;
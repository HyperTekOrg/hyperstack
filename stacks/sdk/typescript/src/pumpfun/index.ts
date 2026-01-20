export interface PumpfunTokenEvents {
  buys?: EventWrapper<Buy>[];
  create?: Create | null;
  sells?: EventWrapper<Sell>[];
}

export interface PumpfunTokenId {
  bondingCurve?: string;
  mint?: string;
}

export interface PumpfunTokenInfo {
  isComplete?: boolean | null;
  name?: string | null;
  symbol?: string | null;
  uri?: string | null;
}

export interface PumpfunTokenReserves {
  currentPriceSol?: number | null;
  marketCapSol?: number | null;
  realSolReserves?: number | null;
  realTokenReserves?: number | null;
  tokenTotalSupply?: number | null;
  virtualSolReserves?: number | null;
  virtualTokenReserves?: number | null;
}

export interface PumpfunTokenTrading {
  averageTradeSize?: number | null;
  buyCount?: number | null;
  largestTrade?: number | null;
  lastTradePrice?: number | null;
  lastTradeTimestamp?: number | null;
  lastWhaleAddress?: string | null;
  sellCount?: number | null;
  smallestTrade?: number | null;
  totalBuyVolume?: number | null;
  totalSellVolume?: number | null;
  totalTrades?: number | null;
  totalVolume?: number | null;
  uniqueTraders?: number | null;
  whaleTradeCount?: number | null;
}

export interface PumpfunToken {
  events?: PumpfunTokenEvents;
  id?: PumpfunTokenId;
  info?: PumpfunTokenInfo;
  reserves?: PumpfunTokenReserves;
  trading?: PumpfunTokenTrading;
  bondingCurveSnapshot?: BondingCurve | null;
}

export interface Create {
  mint?: string;
  mintAuthority?: string;
  bondingCurve?: string;
  associatedBondingCurve?: string;
  global?: string;
  mplTokenMetadata?: string;
  metadata?: string;
  user?: string;
  systemProgram?: string;
  tokenProgram?: string;
  associatedTokenProgram?: string;
  rent?: string;
  eventAuthority?: string;
  program?: string;
  name?: string;
  symbol?: string;
  uri?: string;
  creator?: string;
}

export interface Buy {
  global?: string;
  feeRecipient?: string;
  mint?: string;
  bondingCurve?: string;
  associatedBondingCurve?: string;
  associatedUser?: string;
  user?: string;
  systemProgram?: string;
  tokenProgram?: string;
  creatorVault?: string;
  eventAuthority?: string;
  program?: string;
  amount?: number;
  maxSolCost?: number;
}

export interface Sell {
  global?: string;
  feeRecipient?: string;
  mint?: string;
  bondingCurve?: string;
  associatedBondingCurve?: string;
  associatedUser?: string;
  user?: string;
  systemProgram?: string;
  creatorVault?: string;
  tokenProgram?: string;
  eventAuthority?: string;
  program?: string;
  amount?: number;
  minSolOutput?: number;
}

export interface BondingCurve {
  virtualTokenReserves?: number;
  virtualSolReserves?: number;
  realTokenReserves?: number;
  realSolReserves?: number;
  tokenTotalSupply?: number;
  complete?: boolean;
  creator?: string;
}

export interface BuysEvent {
  amount: number;
  maxSolCost: number;
}

export interface CreateEvent { }

export interface SellsEvent { }

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

/** Helper to create typed state view definitions */
function stateView<T>(view: string): ViewDef<T, 'state'> {
  return { mode: 'state', view } as const;
}

/** Helper to create typed list view definitions */
function listView<T>(view: string): ViewDef<T, 'list'> {
  return { mode: 'list', view } as const;
}

// ============================================================================
// Stack Definition
// ============================================================================

/** Stack definition for PumpfunToken */
export const PUMPFUNTOKEN_STACK = {
  name: 'pumpfun-token',
  views: {
    pumpfunToken: {
      state: stateView<PumpfunToken>('PumpfunToken/state'),
      list: listView<PumpfunToken>('PumpfunToken/list'),
    },
  },
} as const;

/** Type alias for the stack */
export type PumpfunTokenStack = typeof PUMPFUNTOKEN_STACK;

/** Default export for convenience */
export default PUMPFUNTOKEN_STACK;

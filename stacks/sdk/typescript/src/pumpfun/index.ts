export interface PumpfunTokenId {
  mint?: string;
  bondingCurve?: string;
}

export interface PumpfunTokenInfo {
  name?: string | null;
  symbol?: string | null;
  uri?: string | null;
  isComplete?: boolean | null;
}

export interface PumpfunTokenReserves {
  virtualTokenReserves?: number | null;
  virtualSolReserves?: number | null;
  realTokenReserves?: number | null;
  realSolReserves?: number | null;
  tokenTotalSupply?: number | null;
  currentPriceSol?: number | null;
  marketCapSol?: number | null;
}

export interface PumpfunTokenTrading {
  totalBuyVolume?: number | null;
  totalSellVolume?: number | null;
  totalTrades?: number | null;
  buyCount?: number | null;
  sellCount?: number | null;
  uniqueTraders?: number | null;
  largestTrade?: number | null;
  smallestTrade?: number | null;
  lastTradeTimestamp?: number | null;
  lastTradePrice?: number | null;
  whaleTradeCount?: number | null;
  lastWhaleAddress?: string | null;
  totalVolume?: number | null;
  averageTradeSize?: number | null;
}

export interface PumpfunTokenEvents {
  create?: Create | null;
  buys?: EventWrapper<Buy>[];
  sells?: EventWrapper<Sell>[];
}

export interface PumpfunToken {
  id?: PumpfunTokenId;
  info?: PumpfunTokenInfo;
  reserves?: PumpfunTokenReserves;
  trading?: PumpfunTokenTrading;
  events?: PumpfunTokenEvents;
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

export interface EventWrapper<T> {
  timestamp: number;
  data: T;
  slot?: number;
  signature?: string;
}

export interface ViewDef<T, TMode extends 'state' | 'list'> {
  readonly mode: TMode;
  readonly view: string;
  readonly _entity?: T;
}

function stateView<T>(view: string): ViewDef<T, 'state'> {
  return { mode: 'state', view } as const;
}

function listView<T>(view: string): ViewDef<T, 'list'> {
  return { mode: 'list', view } as const;
}

export const PUMPFUN_STACK = {
  name: 'pumpfun-token',
  views: {
    pumpfunToken: {
      state: stateView<PumpfunToken>('PumpfunToken/state'),
      list: listView<PumpfunToken>('PumpfunToken/list'),
    },
  },
} as const;

export type PumpfunStack = typeof PUMPFUN_STACK;

export default PUMPFUN_STACK;

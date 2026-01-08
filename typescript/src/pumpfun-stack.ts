import { defineStack, createStateView, createListView } from './index';

export interface PumpToken {
  mint: string;
  name?: string;
  symbol?: string;
  uri?: string;
  virtual_token_reserves?: bigint;
  virtual_sol_reserves?: bigint;
  real_token_reserves?: bigint;
  real_sol_reserves?: bigint;
  token_total_supply?: bigint;
  complete?: boolean;
  creator?: string;
  created_at?: number;
}

export interface Trade {
  mint: string;
  timestamp: number;
  wallet: string;
  direction: string;
  amount_sol: number;
  token_amount?: bigint;
}

export interface TokenHolding {
  mint: string;
  balance_tokens: number;
  value_sol: number;
}

export interface WalletHoldings {
  wallet: string;
  holdings: Record<string, TokenHolding>;
}

const transformPumpToken = (data: any): PumpToken => ({
  mint: data.mint,
  name: data.name ?? undefined,
  symbol: data.symbol ?? undefined,
  uri: data.uri ?? undefined,
  virtual_token_reserves: data.virtual_token_reserves != null ? BigInt(data.virtual_token_reserves) : undefined,
  virtual_sol_reserves: data.virtual_sol_reserves != null ? BigInt(data.virtual_sol_reserves) : undefined,
  real_token_reserves: data.real_token_reserves != null ? BigInt(data.real_token_reserves) : undefined,
  real_sol_reserves: data.real_sol_reserves != null ? BigInt(data.real_sol_reserves) : undefined,
  token_total_supply: data.token_total_supply != null ? BigInt(data.token_total_supply) : undefined,
  complete: data.complete ?? undefined,
  creator: data.creator ?? undefined,
  created_at: data.created_at ?? undefined,
});

const transformTrade = (data: any): Trade => ({
  mint: data.mint,
  timestamp: data.timestamp,
  wallet: data.wallet,
  direction: data.direction,
  amount_sol: data.amount_sol,
  token_amount: data.token_amount != null ? BigInt(data.token_amount) : undefined,
});

const transformWalletHoldings = (data: any): WalletHoldings => ({
  wallet: data.wallet,
  holdings: data.holdings || {},
});

export const PUMPFUN_STACK = defineStack({
  name: 'pumpfun',

  views: {
    tokens: {
      list: createListView<PumpToken>('tokens/list', {
        transform: transformPumpToken
      }),
      state: createStateView<PumpToken>('tokens/state', {
        transform: transformPumpToken
      })
    },

    trades: {
      list: createListView<Trade>('trades/list', {
        transform: transformTrade
      })
    },

    walletHoldings: {
      list: createListView<WalletHoldings>('walletholdings/list', {
        transform: transformWalletHoldings
      })
    }
  },

  transactions: {},

  helpers: {
    formatPrice: (value: number) => `$${value.toFixed(4)}`,
    formatSupply: (supply: bigint) => supply.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ','),
    formatSol: (value: number) => `${value.toFixed(6)} SOL`,
    calculateMarketCap: (price: number, supply: bigint) => price * Number(supply),
    getBondingCurvePrice: (token: PumpToken): number | undefined => {
      if (token.virtual_sol_reserves && token.virtual_token_reserves) {
        return Number(token.virtual_sol_reserves) / Number(token.virtual_token_reserves);
      }
      return undefined;
    }
  }
});

export type PumpfunStack = typeof PUMPFUN_STACK;

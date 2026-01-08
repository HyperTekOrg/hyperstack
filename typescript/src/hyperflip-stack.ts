import { defineStack, createStateView, createListView } from './index';

export interface GameId {
  globalCount: number;
  gameId: number;
}

export interface GameStatus {
  current?: string;
  createdAt?: number;
  activatedAt?: number;
  settledAt?: number;
}

export interface GameMetrics {
  totalVolume?: number;
  totalEv?: number;
  betCount?: number;
  uniquePlayers?: number;
  totalFeesCollected?: number;
  totalPayoutsDistributed?: number;
  houseProfitLoss?: number;
  claimRate?: number;
}

export interface GameEvent {
  timestamp: number;
  data: any;
}

export interface GameEvents {
  created?: GameEvent;
  activated?: GameEvent;
  betsPlaced: GameEvent[];
  bettingClosed?: GameEvent;
  settled?: GameEvent;
  payoutsClaimed: GameEvent[];
}

export interface SettlementGame {
  id: GameId;
  status: GameStatus;
  metrics: GameMetrics;
  events: GameEvents;
}

export const HYPERFLIP_STACK = defineStack({
  name: 'settlement-game',

  views: {
    game: {
      state: createStateView<SettlementGame>('SettlementGame/state', {
        transform: (data: any) => {
        }
      }),
      list: createListView<SettlementGame>('SettlementGame/list', {
        transform: (data: any) => {
        }
      })
    }
  },

  helpers: {
    formatVolume: (volume: number) => volume.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ','),
    formatTimestamp: (timestamp: number) => new Date(timestamp * 1000).toISOString(),
    calculateWinRate: (metrics: GameMetrics) => {
      if (!metrics.totalPayoutsDistributed || !metrics.totalVolume) return 0;
      return metrics.totalPayoutsDistributed / metrics.totalVolume;
    },
    isActive: (status: GameStatus) => status.current === 'Active',
    isSettled: (status: GameStatus) => status.current === 'Settled',
    formatGameId: (id: GameId) => `${id.globalCount}-${id.gameId}`
  }
});

export type HyperflipStack = typeof HYPERFLIP_STACK;

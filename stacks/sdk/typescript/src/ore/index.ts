export interface OreRoundEntropy {
  entropy_end_at?: number | null;
  entropy_samples?: number | null;
  entropy_seed?: string | null;
  entropy_slot_hash?: string | null;
  entropy_start_at?: number | null;
  entropy_value?: string | null;
  entropy_var_address?: string | null;
}

export interface OreRoundId {
  round_address?: string | null;
  round_id?: number | null;
}

export interface OreRoundMetrics {
  checkpoint_count?: number | null;
  deploy_count?: number | null;
  total_deployed_sol?: number | null;
}

export interface OreRoundResults {
  did_hit_motherlode?: boolean | null;
  rent_payer?: string | null;
  rng?: number | null;
  slot_hash?: string | null;
  top_miner?: string | null;
  top_miner_reward?: number | null;
  winning_square?: number | null;
}

export interface OreRoundState {
  expires_at?: number | null;
  motherlode?: number | null;
  total_deployed?: number | null;
  total_vaulted?: number | null;
  total_winnings?: number | null;
}

export interface OreRound {
  entropy?: OreRoundEntropy;
  id?: OreRoundId;
  metrics?: OreRoundMetrics;
  results?: OreRoundResults;
  state?: OreRoundState;
  round_snapshot?: Round | null;
  entropy_snapshot?: Var | null;
}

export interface Round {
  id?: number;
  deployed?: number[];
  slot_hash?: number[];
  count?: number[];
  expires_at?: number;
  motherlode?: number;
  rent_payer?: string;
  top_miner?: string;
  top_miner_reward?: number;
  total_deployed?: number;
  total_miners?: number;
  total_vaulted?: number;
  total_winnings?: number;
}

export interface Var {
  authority?: string;
  id?: number;
  provider?: string;
  commit?: number[];
  seed?: number[];
  slot_hash?: number[];
  value?: number[];
  samples?: number;
  is_auto?: number;
  start_at?: number;
  end_at?: number;
}

export interface OreTreasuryId {
  address?: string | null;
}

export interface OreTreasuryState {
  balance?: number | null;
  motherlode?: number | null;
  total_refined?: number | null;
  total_staked?: number | null;
  total_unclaimed?: number | null;
}

export interface OreTreasury {
  id?: OreTreasuryId;
  state?: OreTreasuryState;
  treasury_snapshot?: Treasury | null;
}

export interface Treasury {
  balance?: number;
  buffer_a?: number;
  motherlode?: number;
  miner_rewards_factor?: Record<string, any>;
  stake_rewards_factor?: Record<string, any>;
  buffer_b?: number;
  total_refined?: number;
  total_staked?: number;
  total_unclaimed?: number;
}

export interface OreMinerAutomation {
  amount?: number | null;
  balance?: number | null;
  executor?: string | null;
  fee?: number | null;
  mask?: number | null;
  reload?: number | null;
  strategy?: number | null;
}

export interface OreMinerId {
  authority?: string | null;
  automation_address?: string | null;
  miner_address?: string | null;
}

export interface OreMinerRewards {
  lifetime_deployed?: number | null;
  lifetime_rewards_ore?: number | null;
  lifetime_rewards_sol?: number | null;
  refined_ore?: number | null;
  rewards_ore?: number | null;
  rewards_sol?: number | null;
}

export interface OreMinerState {
  checkpoint_fee?: number | null;
  checkpoint_id?: number | null;
  last_claim_ore_at?: number | null;
  last_claim_sol_at?: number | null;
  round_id?: number | null;
}

export interface OreMiner {
  automation?: OreMinerAutomation;
  id?: OreMinerId;
  rewards?: OreMinerRewards;
  state?: OreMinerState;
  miner_snapshot?: Miner | null;
  automation_snapshot?: Automation | null;
}

export interface Miner {
  authority?: string;
  deployed?: number[];
  cumulative?: number[];
  checkpoint_fee?: number;
  checkpoint_id?: number;
  last_claim_ore_at?: number;
  last_claim_sol_at?: number;
  rewards_factor?: Record<string, any>;
  rewards_sol?: number;
  rewards_ore?: number;
  refined_ore?: number;
  round_id?: number;
  lifetime_rewards_sol?: number;
  lifetime_rewards_ore?: number;
  lifetime_deployed?: number;
}

export interface Automation {
  amount?: number;
  authority?: string;
  balance?: number;
  executor?: string;
  fee?: number;
  strategy?: number;
  mask?: number;
  reload?: number;
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

/** Stack definition for OreStream with 3 entities */
export const ORE_STREAM_STACK = {
  name: 'ore-stream',
  url: 'wss://ore.stack.usehyperstack.com',
  views: {
    OreRound: {
      state: stateView<OreRound>('OreRound/state'),
      list: listView<OreRound>('OreRound/list'),
      latest: listView<OreRound>('OreRound/latest'),
    },
    OreTreasury: {
      state: stateView<OreTreasury>('OreTreasury/state'),
      list: listView<OreTreasury>('OreTreasury/list'),
    },
    OreMiner: {
      state: stateView<OreMiner>('OreMiner/state'),
      list: listView<OreMiner>('OreMiner/list'),
    },
  },
} as const;

/** Type alias for the stack */
export type OreStreamStack = typeof ORE_STREAM_STACK;

/** Entity types in this stack */
export type OreStreamEntity = OreRound | OreTreasury | OreMiner;

/** Default export for convenience */
export default ORE_STREAM_STACK;
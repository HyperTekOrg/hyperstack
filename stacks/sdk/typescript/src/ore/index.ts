

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
  id?: OreRoundId;
  metrics?: OreRoundMetrics;
  results?: OreRoundResults;
  state?: OreRoundState;
  round_snapshot?: Round | null;
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

/** Stack definition for OreRound */
export const OREROUND_STACK = {
  name: 'ore-round',
  views: {
    OreRound: {
      state: stateView<OreRound>('OreRound/state'),
      list: listView<OreRound>('OreRound/list'),
      latest: listView<OreRound>('OreRound/latest'),
    },
  },
} as const;

/** Type alias for the stack */
export type OreRoundStack = typeof OREROUND_STACK;

/** Default export for convenience */
export default OREROUND_STACK;


export interface OreRoundId {
  roundAddress?: string | null;
  roundId?: number | null;
}

export interface OreRoundMetrics {
  checkpointCount?: number | null;
  deployCount?: number | null;
  totalDeployedSol?: number | null;
}

export interface OreRoundResults {
  didHitMotherlode?: boolean | null;
  rentPayer?: string | null;
  rng?: number | null;
  slotHash?: string | null;
  topMiner?: string | null;
  topMinerReward?: number | null;
  winningSquare?: number | null;
}

export interface OreRoundState {
  expiresAt?: number | null;
  motherlode?: number | null;
  totalDeployed?: number | null;
  totalVaulted?: number | null;
  totalWinnings?: number | null;
}

export interface OreRound {
  id?: OreRoundId;
  metrics?: OreRoundMetrics;
  results?: OreRoundResults;
  state?: OreRoundState;
  roundSnapshot?: Round | null;
}

export interface Round {
  id?: number;
  deployed?: number[];
  slotHash?: number[];
  count?: number[];
  expiresAt?: number;
  motherlode?: number;
  rentPayer?: string;
  topMiner?: string;
  topMinerReward?: number;
  totalDeployed?: number;
  totalVaulted?: number;
  totalWinnings?: number;
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

/** Helper to create typed derived view definitions */
function derivedView<T>(view: string, output: 'single' | 'collection'): ViewDef<T, 'state' | 'list'> {
  return { mode: output === 'single' ? 'state' : 'list', view } as const;
}

// ============================================================================
// Stack Definition
// ============================================================================

/** Stack definition for OreRound */
export const OREROUND_STACK = {
  name: 'ore-round',
  views: {
    oreRound: {
      state: stateView<OreRound>('OreRound/state'),
      list: listView<OreRound>('OreRound/list'),
      latest: derivedView<OreRound>('OreRound/latest', 'single'),
    },
  },
} as const;

/** Type alias for the stack */
export type OreRoundStack = typeof OREROUND_STACK;

/** Default export for convenience */
export default OREROUND_STACK;
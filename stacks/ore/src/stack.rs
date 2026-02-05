use hyperstack::prelude::*;

#[hyperstack(idl = ["idl/ore.json", "idl/entropy.json"])]
pub mod ore_stream {
    use hyperstack::macros::Stream;
    use hyperstack::resolvers::TokenMetadata;

    use serde::{Deserialize, Serialize};

    #[entity(name = "OreRound")]
    #[view(name = "latest", sort_by = "id.round_id", order = "desc")]
    pub struct OreRound {
        pub id: RoundId,
        pub state: RoundState,
        pub results: RoundResults,
        pub metrics: RoundMetrics,
        pub treasury: RoundTreasury,
        pub entropy: EntropyState,
        #[resolve(address = "oreoU2P8bN6jkk3jbaiVxYnG1dCXcYxwhwyK9jSybcp")]
        pub ore_metadata: Option<TokenMetadata>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundId {
        #[map(ore_sdk::accounts::Round::id, primary_key, strategy = SetOnce)]
        pub round_id: u64,

        #[map(ore_sdk::accounts::Round::__account_address, lookup_index, strategy = SetOnce)]
        pub round_address: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundState {
        #[map(ore_sdk::accounts::Round::expires_at, strategy = LastWrite)]
        pub expires_at: Option<u64>,

        #[map(ore_sdk::accounts::Round::motherlode, strategy = LastWrite)]
        pub motherlode: Option<u64>,

        #[computed(state.motherlode.ui_amount(ore_metadata.decimals))]
        pub motherlode_ui: Option<u64>,

        #[map(ore_sdk::accounts::Round::total_deployed, strategy = LastWrite)]
        pub total_deployed: Option<u64>,

        #[computed(state.total_deployed.ui_amount(9))]
        pub total_deployed_ui: Option<u64>,

        #[map(ore_sdk::accounts::Round::total_vaulted, strategy = LastWrite)]
        pub total_vaulted: Option<u64>,

        #[computed(state.total_vaulted.ui_amount(9))]
        pub total_vaulted_ui: Option<u64>,

        #[map(ore_sdk::accounts::Round::total_winnings, strategy = LastWrite)]
        pub total_winnings: Option<u64>,

        #[computed(state.total_winnings.ui_amount(9))]
        pub total_winnings_ui: Option<u64>,

        #[map(ore_sdk::accounts::Round::total_miners, strategy = LastWrite)]
        pub total_miners: Option<u64>,

        // Per-square deployed SOL amounts (25 squares in 5x5 grid)
        #[map(ore_sdk::accounts::Round::deployed, strategy = LastWrite)]
        pub deployed_per_square: Option<Vec<u64>>,

        #[computed(state.deployed_per_square.map(|x| x.ui_amount(9)))]
        pub deployed_per_square_ui: Option<Vec<f64>>,

        // Per-square miner counts (25 squares in 5x5 grid)
        #[map(ore_sdk::accounts::Round::count, strategy = LastWrite)]
        pub count_per_square: Option<Vec<u64>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundResults {
        #[map(ore_sdk::accounts::Round::top_miner, strategy = LastWrite, transform = Base58Encode)]
        pub top_miner: Option<String>,

        #[map(ore_sdk::accounts::Round::top_miner_reward, strategy = LastWrite)]
        pub top_miner_reward: Option<u64>,

        #[computed(results.top_miner_reward.ui_amount(ore_metadata.decimals))]
        pub top_miner_reward_ui: Option<f64>,

        #[map(ore_sdk::accounts::Round::rent_payer, strategy = LastWrite, transform = Base58Encode)]
        pub rent_payer: Option<String>,

        #[map(ore_sdk::accounts::Round::slot_hash, strategy = LastWrite, transform = Base58Encode)]
        pub slot_hash: Option<String>,

        #[computed(
            let hash = entropy.entropy_value_bytes.to_bytes();
            if (hash.len() as u64) != 32 {
                None
            } else {
                let all_zeros = hash == [0u8; 32];
                let all_ff = hash == [0xFFu8; 32];
                if all_zeros || all_ff {
                    None
                } else {
                    let r1 = u64::from_le_bytes(hash[0..8]);
                    let r2 = u64::from_le_bytes(hash[8..16]);
                    let r3 = u64::from_le_bytes(hash[16..24]);
                    let r4 = u64::from_le_bytes(hash[24..32]);
                    Some(r1 ^ r2 ^ r3 ^ r4)
                }
            }
        )]
        pub rng: Option<u64>,

        #[computed(results.rng.map(|r| r % 25))]
        pub winning_square: Option<u64>,

        #[computed(results.rng.map(|r| r.reverse_bits() % 625 == 0))]
        pub did_hit_motherlode: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundMetrics {
        // Count of deploy instructions for this round
        #[aggregate(from = ore_sdk::instructions::Deploy, strategy = Count, lookup_by = accounts::round)]
        pub deploy_count: Option<u64>,

        // Sum of all deployed SOL amounts
        #[aggregate(from = ore_sdk::instructions::Deploy, field = amount, strategy = Sum, lookup_by = accounts::round)]
        pub total_deployed_sol: Option<u64>,

        #[computed(metrics.total_deployed_sol.ui_amount(9))]
        pub total_deployed_sol_ui: Option<u64>,

        // Count of checkpoint instructions for this round
        #[aggregate(from = ore_sdk::instructions::Checkpoint, strategy = Count, lookup_by = accounts::round)]
        pub checkpoint_count: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundTreasury {
        #[map(ore_sdk::accounts::Treasury::motherlode,
              lookup_index(register_from = [
                  (ore_sdk::instructions::Reset, accounts::treasury, accounts::roundNext)
              ]),
              stop = ore_sdk::instructions::Reset,
              stop_lookup_by = accounts::round,
              strategy = SetOnce)]
        pub motherlode: Option<u64>,

        #[computed(treasury.motherlode.ui_amount(ore_metadata.decimals))]
        pub motherlode_ui: Option<f64>,
    }

    // ========================================================================
    // Entropy — Cross-program randomness state from the Entropy program
    // Linked to OreRound via Deploy/Reset instructions that reference both
    // accounts::round and accounts::entropyVar in the same transaction.
    // ========================================================================

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct EntropyState {
        #[map(entropy_sdk::accounts::Var::value,
              lookup_index(register_from = [
                  (ore_sdk::instructions::Deploy, accounts::entropyVar, accounts::round),
                  (ore_sdk::instructions::Reset, accounts::entropyVar, accounts::round)
              ]),
              when = entropy_sdk::instructions::Reveal,
              condition = "value != ZERO_32",
              strategy = LastWrite,
              transform = Base58Encode)]
        pub entropy_value: Option<String>,

        // Raw bytes for computed field (not emitted to clients)
        #[map(entropy_sdk::accounts::Var::value,
              when = entropy_sdk::instructions::Reveal,
              condition = "value != ZERO_32",
              strategy = LastWrite,
              emit = false)]
        pub entropy_value_bytes: Option<Vec<u8>>,

        #[map(entropy_sdk::accounts::Var::seed, strategy = LastWrite, transform = Base58Encode)]
        pub entropy_seed: Option<String>,

        #[map(entropy_sdk::accounts::Var::slot_hash, strategy = LastWrite, transform = Base58Encode)]
        pub entropy_slot_hash: Option<String>,

        #[map(entropy_sdk::accounts::Var::start_at, strategy = LastWrite)]
        pub entropy_start_at: Option<u64>,

        #[map(entropy_sdk::accounts::Var::end_at, strategy = LastWrite)]
        pub entropy_end_at: Option<u64>,

        #[map(entropy_sdk::accounts::Var::samples, strategy = LastWrite)]
        pub entropy_samples: Option<u64>,

        #[map(entropy_sdk::accounts::Var::__account_address, strategy = SetOnce)]
        pub entropy_var_address: Option<String>,
    }

    // ========================================================================
    // Treasury Entity — Singleton protocol-wide state
    // ========================================================================

    #[entity(name = "OreTreasury")]
    pub struct OreTreasury {
        pub id: TreasuryId,
        pub state: TreasuryState,

        #[snapshot(strategy = LastWrite)]
        pub treasury_snapshot: Option<ore_sdk::accounts::Treasury>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TreasuryId {
        #[map(ore_sdk::accounts::Treasury::__account_address, primary_key, strategy = SetOnce)]
        pub address: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TreasuryState {
        #[map(ore_sdk::accounts::Treasury::balance, strategy = LastWrite)]
        pub balance: Option<u64>,

        #[map(ore_sdk::accounts::Treasury::motherlode, strategy = LastWrite)]
        pub motherlode: Option<u64>,

        /// Motherlode formatted in ORE tokens (11 decimals)
        /// Uses resolver-computed ui_amount method for decimal conversion
        #[computed(state.motherlode.ui_amount(11))]
        pub motherlode_ui: Option<f64>,

        #[map(ore_sdk::accounts::Treasury::total_refined, strategy = LastWrite)]
        pub total_refined: Option<u64>,

        /// Total refined formatted in ORE tokens (11 decimals)
        #[computed(state.total_refined.ui_amount(11))]
        pub total_refined_ui: Option<f64>,

        #[map(ore_sdk::accounts::Treasury::total_staked, strategy = LastWrite)]
        pub total_staked: Option<u64>,

        /// Total staked formatted in ORE tokens (11 decimals)
        #[computed(state.total_staked.ui_amount(11))]
        pub total_staked_ui: Option<f64>,

        #[map(ore_sdk::accounts::Treasury::total_unclaimed, strategy = LastWrite)]
        pub total_unclaimed: Option<u64>,

        /// Total unclaimed formatted in ORE tokens (11 decimals)
        #[computed(state.total_unclaimed.ui_amount(11))]
        pub total_unclaimed_ui: Option<f64>,
    }

    // ========================================================================
    // Miner Entity — Per-user mining state across all rounds
    // ========================================================================

    #[entity(name = "OreMiner")]
    pub struct OreMiner {
        pub id: MinerId,
        pub rewards: MinerRewards,
        pub state: MinerState,
        pub automation: MinerAutomation,

        #[snapshot(strategy = LastWrite, transforms = [(authority, Base58Encode)])]
        pub miner_snapshot: Option<ore_sdk::accounts::Miner>,

        #[snapshot(strategy = LastWrite, transforms = [(authority, Base58Encode), (executor, Base58Encode)])]
        pub automation_snapshot: Option<ore_sdk::accounts::Automation>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerId {
        // Both Miner and Automation accounts share authority as the identity key
        #[map([ore_sdk::accounts::Miner::authority, ore_sdk::accounts::Automation::authority], primary_key, strategy = SetOnce, transform = Base58Encode)]
        pub authority: String,

        #[map(ore_sdk::accounts::Miner::__account_address, lookup_index, strategy = SetOnce)]
        pub miner_address: String,

        #[map(ore_sdk::accounts::Automation::__account_address, strategy = SetOnce)]
        pub automation_address: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerRewards {
        #[map(ore_sdk::accounts::Miner::rewards_sol, strategy = LastWrite)]
        pub rewards_sol: Option<u64>,

        #[map(ore_sdk::accounts::Miner::rewards_ore, strategy = LastWrite)]
        pub rewards_ore: Option<u64>,

        #[map(ore_sdk::accounts::Miner::refined_ore, strategy = LastWrite)]
        pub refined_ore: Option<u64>,

        #[map(ore_sdk::accounts::Miner::lifetime_rewards_sol, strategy = LastWrite)]
        pub lifetime_rewards_sol: Option<u64>,

        #[map(ore_sdk::accounts::Miner::lifetime_rewards_ore, strategy = LastWrite)]
        pub lifetime_rewards_ore: Option<u64>,

        #[map(ore_sdk::accounts::Miner::lifetime_deployed, strategy = LastWrite)]
        pub lifetime_deployed: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerState {
        #[map(ore_sdk::accounts::Miner::round_id, strategy = LastWrite)]
        pub round_id: Option<u64>,

        #[map(ore_sdk::accounts::Miner::checkpoint_id, strategy = LastWrite)]
        pub checkpoint_id: Option<u64>,

        #[map(ore_sdk::accounts::Miner::checkpoint_fee, strategy = LastWrite)]
        pub checkpoint_fee: Option<u64>,

        #[map(ore_sdk::accounts::Miner::last_claim_ore_at, strategy = LastWrite)]
        pub last_claim_ore_at: Option<i64>,

        #[map(ore_sdk::accounts::Miner::last_claim_sol_at, strategy = LastWrite)]
        pub last_claim_sol_at: Option<i64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerAutomation {
        #[map(ore_sdk::accounts::Automation::amount, strategy = LastWrite)]
        pub amount: Option<u64>,

        #[map(ore_sdk::accounts::Automation::balance, strategy = LastWrite)]
        pub balance: Option<u64>,

        #[map(ore_sdk::accounts::Automation::executor, strategy = LastWrite, transform = Base58Encode)]
        pub executor: Option<String>,

        #[map(ore_sdk::accounts::Automation::fee, strategy = LastWrite)]
        pub fee: Option<u64>,

        #[map(ore_sdk::accounts::Automation::strategy, strategy = LastWrite)]
        pub strategy: Option<u64>,

        #[map(ore_sdk::accounts::Automation::mask, strategy = LastWrite)]
        pub mask: Option<u64>,

        #[map(ore_sdk::accounts::Automation::reload, strategy = LastWrite)]
        pub reload: Option<u64>,
    }
}

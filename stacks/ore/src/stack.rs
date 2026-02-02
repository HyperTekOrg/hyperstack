use hyperstack::prelude::*;

#[hyperstack(idl = "idl/ore.json")]
pub mod ore_stream {
    use hyperstack::macros::Stream;

    use serde::{Deserialize, Serialize};

    #[entity(name = "OreRound")]
    #[view(name = "latest", sort_by = "id.round_id", order = "desc")]
    pub struct OreRound {
        pub id: RoundId,
        pub state: RoundState,
        pub results: RoundResults,
        pub metrics: RoundMetrics,

        // Snapshot the entire Round account state with field transformations
        // - rent_payer: Base58Encode - Converts the pubkey to readable base58 string
        // - top_miner: Base58Encode - Converts the pubkey to readable base58 string
        #[snapshot(strategy = LastWrite, transforms = [(rent_payer, Base58Encode), (top_miner, Base58Encode)])]
        pub round_snapshot: Option<generated_sdk::accounts::Round>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundId {
        #[map(generated_sdk::accounts::Round::id, primary_key, strategy = SetOnce)]
        pub round_id: u64,

        #[map(generated_sdk::accounts::Round::__account_address, lookup_index, strategy = SetOnce)]
        pub round_address: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundState {
        #[map(generated_sdk::accounts::Round::expires_at, strategy = LastWrite)]
        pub expires_at: Option<u64>,

        #[map(generated_sdk::accounts::Round::motherlode, strategy = LastWrite)]
        pub motherlode: Option<u64>,

        #[map(generated_sdk::accounts::Round::total_deployed, strategy = LastWrite)]
        pub total_deployed: Option<u64>,

        #[map(generated_sdk::accounts::Round::total_vaulted, strategy = LastWrite)]
        pub total_vaulted: Option<u64>,

        #[map(generated_sdk::accounts::Round::total_winnings, strategy = LastWrite)]
        pub total_winnings: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundResults {
        #[map(generated_sdk::accounts::Round::top_miner, strategy = LastWrite, transform = Base58Encode)]
        pub top_miner: Option<String>,

        #[map(generated_sdk::accounts::Round::top_miner_reward, strategy = LastWrite)]
        pub top_miner_reward: Option<u64>,

        #[map(generated_sdk::accounts::Round::rent_payer, strategy = LastWrite, transform = Base58Encode)]
        pub rent_payer: Option<String>,

        #[map(generated_sdk::accounts::Round::slot_hash, strategy = LastWrite, transform = Base58Encode)]
        pub slot_hash: Option<String>,

        // Computed field: Calculate RNG from slot_hash bytes
        // XOR the 4 u64 values from the 32-byte hash to get a single random number
        #[computed(
            let hash = round_snapshot.data.slot_hash.to_bytes();
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
        )]
        pub rng: Option<u64>,

        // Computed field: Winning square = rng mod 25 (5x5 grid)
        #[computed(results.rng.map(|r| r % 25))]
        pub winning_square: Option<u64>,

        // Computed field: Did the round hit the motherlode (1/625 chance)
        #[computed(results.rng.map(|r| r.reverse_bits() % 625 == 0))]
        pub did_hit_motherlode: Option<bool>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct RoundMetrics {
        // Count of deploy instructions for this round
        #[aggregate(from = generated_sdk::instructions::Deploy, strategy = Count, lookup_by = accounts::round)]
        pub deploy_count: Option<u64>,

        // Sum of all deployed SOL amounts
        #[aggregate(from = generated_sdk::instructions::Deploy, field = amount, strategy = Sum, lookup_by = accounts::round)]
        pub total_deployed_sol: Option<u64>,

        // Count of checkpoint instructions for this round
        #[aggregate(from = generated_sdk::instructions::Checkpoint, strategy = Count, lookup_by = accounts::round)]
        pub checkpoint_count: Option<u64>,
    }

    // ========================================================================
    // Treasury Entity — Singleton protocol-wide state
    // ========================================================================

    #[entity(name = "OreTreasury")]
    pub struct OreTreasury {
        pub id: TreasuryId,
        pub state: TreasuryState,

        #[snapshot(strategy = LastWrite)]
        pub treasury_snapshot: Option<generated_sdk::accounts::Treasury>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TreasuryId {
        #[map(generated_sdk::accounts::Treasury::__account_address, primary_key, strategy = SetOnce)]
        pub address: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TreasuryState {
        #[map(generated_sdk::accounts::Treasury::balance, strategy = LastWrite)]
        pub balance: Option<u64>,

        #[map(generated_sdk::accounts::Treasury::motherlode, strategy = LastWrite)]
        pub motherlode: Option<u64>,

        #[map(generated_sdk::accounts::Treasury::total_refined, strategy = LastWrite)]
        pub total_refined: Option<u64>,

        #[map(generated_sdk::accounts::Treasury::total_staked, strategy = LastWrite)]
        pub total_staked: Option<u64>,

        #[map(generated_sdk::accounts::Treasury::total_unclaimed, strategy = LastWrite)]
        pub total_unclaimed: Option<u64>,
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
        pub miner_snapshot: Option<generated_sdk::accounts::Miner>,

        #[snapshot(strategy = LastWrite, transforms = [(authority, Base58Encode), (executor, Base58Encode)])]
        pub automation_snapshot: Option<generated_sdk::accounts::Automation>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerId {
        // Both Miner and Automation accounts share authority as the identity key
        #[map([generated_sdk::accounts::Miner::authority, generated_sdk::accounts::Automation::authority], primary_key, strategy = SetOnce, transform = Base58Encode)]
        pub authority: String,

        #[map(generated_sdk::accounts::Miner::__account_address, lookup_index, strategy = SetOnce)]
        pub miner_address: String,

        #[map(generated_sdk::accounts::Automation::__account_address, strategy = SetOnce)]
        pub automation_address: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerRewards {
        #[map(generated_sdk::accounts::Miner::rewards_sol, strategy = LastWrite)]
        pub rewards_sol: Option<u64>,

        #[map(generated_sdk::accounts::Miner::rewards_ore, strategy = LastWrite)]
        pub rewards_ore: Option<u64>,

        #[map(generated_sdk::accounts::Miner::refined_ore, strategy = LastWrite)]
        pub refined_ore: Option<u64>,

        #[map(generated_sdk::accounts::Miner::lifetime_rewards_sol, strategy = LastWrite)]
        pub lifetime_rewards_sol: Option<u64>,

        #[map(generated_sdk::accounts::Miner::lifetime_rewards_ore, strategy = LastWrite)]
        pub lifetime_rewards_ore: Option<u64>,

        #[map(generated_sdk::accounts::Miner::lifetime_deployed, strategy = LastWrite)]
        pub lifetime_deployed: Option<u64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerState {
        #[map(generated_sdk::accounts::Miner::round_id, strategy = LastWrite)]
        pub round_id: Option<u64>,

        #[map(generated_sdk::accounts::Miner::checkpoint_id, strategy = LastWrite)]
        pub checkpoint_id: Option<u64>,

        #[map(generated_sdk::accounts::Miner::checkpoint_fee, strategy = LastWrite)]
        pub checkpoint_fee: Option<u64>,

        #[map(generated_sdk::accounts::Miner::last_claim_ore_at, strategy = LastWrite)]
        pub last_claim_ore_at: Option<i64>,

        #[map(generated_sdk::accounts::Miner::last_claim_sol_at, strategy = LastWrite)]
        pub last_claim_sol_at: Option<i64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct MinerAutomation {
        #[map(generated_sdk::accounts::Automation::amount, strategy = LastWrite)]
        pub amount: Option<u64>,

        #[map(generated_sdk::accounts::Automation::balance, strategy = LastWrite)]
        pub balance: Option<u64>,

        #[map(generated_sdk::accounts::Automation::executor, strategy = LastWrite, transform = Base58Encode)]
        pub executor: Option<String>,

        #[map(generated_sdk::accounts::Automation::fee, strategy = LastWrite)]
        pub fee: Option<u64>,

        #[map(generated_sdk::accounts::Automation::strategy, strategy = LastWrite)]
        pub strategy: Option<u64>,

        #[map(generated_sdk::accounts::Automation::mask, strategy = LastWrite)]
        pub mask: Option<u64>,

        #[map(generated_sdk::accounts::Automation::reload, strategy = LastWrite)]
        pub reload: Option<u64>,
    }
}

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
}

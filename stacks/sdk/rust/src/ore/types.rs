use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRoundId {
    #[serde(default)]
    pub round_id: Option<u64>,
    #[serde(default)]
    pub round_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRoundState {
    #[serde(default)]
    pub expires_at: Option<Option<i64>>,
    #[serde(default)]
    pub motherlode: Option<Option<u64>>,
    #[serde(default)]
    pub total_deployed: Option<Option<u64>>,
    #[serde(default)]
    pub total_vaulted: Option<Option<u64>>,
    #[serde(default)]
    pub total_winnings: Option<Option<u64>>,
    #[serde(default)]
    pub total_miners: Option<Option<u64>>,
    #[serde(default)]
    pub deployed_per_square: Option<Option<Vec<serde_json::Value>>>,
    #[serde(default)]
    pub count_per_square: Option<Option<Vec<serde_json::Value>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRoundResults {
    #[serde(default)]
    pub top_miner: Option<Option<String>>,
    #[serde(default)]
    pub top_miner_reward: Option<Option<u64>>,
    #[serde(default)]
    pub rent_payer: Option<Option<String>>,
    #[serde(default)]
    pub slot_hash: Option<Option<String>>,
    #[serde(default)]
    pub rng: Option<Option<u64>>,
    #[serde(default)]
    pub winning_square: Option<Option<u64>>,
    #[serde(default)]
    pub did_hit_motherlode: Option<Option<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRoundMetrics {
    #[serde(default)]
    pub deploy_count: Option<Option<u64>>,
    #[serde(default)]
    pub total_deployed_sol: Option<Option<u64>>,
    #[serde(default)]
    pub checkpoint_count: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRoundEntropy {
    #[serde(default)]
    pub entropy_value: Option<Option<String>>,
    #[serde(default)]
    pub entropy_seed: Option<Option<String>>,
    #[serde(default)]
    pub entropy_slot_hash: Option<Option<String>>,
    #[serde(default)]
    pub entropy_start_at: Option<Option<i64>>,
    #[serde(default)]
    pub entropy_end_at: Option<Option<i64>>,
    #[serde(default)]
    pub entropy_samples: Option<Option<u64>>,
    #[serde(default)]
    pub entropy_var_address: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreRound {
    #[serde(default)]
    pub id: OreRoundId,
    #[serde(default)]
    pub state: OreRoundState,
    #[serde(default)]
    pub results: OreRoundResults,
    #[serde(default)]
    pub metrics: OreRoundMetrics,
    #[serde(default)]
    pub entropy: OreRoundEntropy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreTreasuryId {
    #[serde(default)]
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreTreasuryState {
    #[serde(default)]
    pub balance: Option<Option<u64>>,
    #[serde(default)]
    pub motherlode: Option<Option<u64>>,
    #[serde(default)]
    pub total_refined: Option<Option<u64>>,
    #[serde(default)]
    pub total_staked: Option<Option<u64>>,
    #[serde(default)]
    pub total_unclaimed: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreTreasury {
    #[serde(default)]
    pub id: OreTreasuryId,
    #[serde(default)]
    pub state: OreTreasuryState,
    #[serde(default)]
    pub treasury_snapshot: Option<Option<serde_json::Value>>,
}



#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Treasury {
    #[serde(default)]
    pub balance: Option<u64>,
    #[serde(default)]
    pub buffer_a: Option<u64>,
    #[serde(default)]
    pub motherlode: Option<u64>,
    #[serde(default)]
    pub miner_rewards_factor: Option<serde_json::Value>,
    #[serde(default)]
    pub stake_rewards_factor: Option<serde_json::Value>,
    #[serde(default)]
    pub buffer_b: Option<u64>,
    #[serde(default)]
    pub total_refined: Option<u64>,
    #[serde(default)]
    pub total_staked: Option<u64>,
    #[serde(default)]
    pub total_unclaimed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreMinerId {
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub miner_address: Option<String>,
    #[serde(default)]
    pub automation_address: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreMinerRewards {
    #[serde(default)]
    pub rewards_sol: Option<Option<u64>>,
    #[serde(default)]
    pub rewards_ore: Option<Option<u64>>,
    #[serde(default)]
    pub refined_ore: Option<Option<u64>>,
    #[serde(default)]
    pub lifetime_rewards_sol: Option<Option<u64>>,
    #[serde(default)]
    pub lifetime_rewards_ore: Option<Option<u64>>,
    #[serde(default)]
    pub lifetime_deployed: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreMinerState {
    #[serde(default)]
    pub round_id: Option<Option<u64>>,
    #[serde(default)]
    pub checkpoint_id: Option<Option<u64>>,
    #[serde(default)]
    pub checkpoint_fee: Option<Option<u64>>,
    #[serde(default)]
    pub last_claim_ore_at: Option<Option<i64>>,
    #[serde(default)]
    pub last_claim_sol_at: Option<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreMinerAutomation {
    #[serde(default)]
    pub amount: Option<Option<u64>>,
    #[serde(default)]
    pub balance: Option<Option<u64>>,
    #[serde(default)]
    pub executor: Option<Option<String>>,
    #[serde(default)]
    pub fee: Option<Option<u64>>,
    #[serde(default)]
    pub strategy: Option<Option<u64>>,
    #[serde(default)]
    pub mask: Option<Option<u64>>,
    #[serde(default)]
    pub reload: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OreMiner {
    #[serde(default)]
    pub id: OreMinerId,
    #[serde(default)]
    pub rewards: OreMinerRewards,
    #[serde(default)]
    pub state: OreMinerState,
    #[serde(default)]
    pub automation: OreMinerAutomation,
    #[serde(default)]
    pub miner_snapshot: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub automation_snapshot: Option<Option<serde_json::Value>>,
}



#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Miner {
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub deployed: Option<Vec<u64>>,
    #[serde(default)]
    pub cumulative: Option<Vec<u64>>,
    #[serde(default)]
    pub checkpoint_fee: Option<u64>,
    #[serde(default)]
    pub checkpoint_id: Option<u64>,
    #[serde(default)]
    pub last_claim_ore_at: Option<i64>,
    #[serde(default)]
    pub last_claim_sol_at: Option<i64>,
    #[serde(default)]
    pub rewards_factor: Option<serde_json::Value>,
    #[serde(default)]
    pub rewards_sol: Option<u64>,
    #[serde(default)]
    pub rewards_ore: Option<u64>,
    #[serde(default)]
    pub refined_ore: Option<u64>,
    #[serde(default)]
    pub round_id: Option<u64>,
    #[serde(default)]
    pub lifetime_rewards_sol: Option<u64>,
    #[serde(default)]
    pub lifetime_rewards_ore: Option<u64>,
    #[serde(default)]
    pub lifetime_deployed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Automation {
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub balance: Option<u64>,
    #[serde(default)]
    pub executor: Option<String>,
    #[serde(default)]
    pub fee: Option<u64>,
    #[serde(default)]
    pub strategy: Option<u64>,
    #[serde(default)]
    pub mask: Option<u64>,
    #[serde(default)]
    pub reload: Option<u64>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWrapper<T> {
    #[serde(default)]
    pub timestamp: i64,
    pub data: T,
    #[serde(default)]
    pub slot: Option<f64>,
    #[serde(default)]
    pub signature: Option<String>,
}

impl<T: Default> Default for EventWrapper<T> {
    fn default() -> Self {
        Self {
            timestamp: 0,
            data: T::default(),
            slot: None,
            signature: None,
        }
    }
}

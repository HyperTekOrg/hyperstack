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
    pub round_snapshot: Option<Option<serde_json::Value>>,
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
pub struct Round {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub deployed: Option<Vec<u64>>,
    #[serde(default)]
    pub slot_hash: Option<Vec<i64>>,
    #[serde(default)]
    pub count: Option<Vec<u64>>,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub motherlode: Option<u64>,
    #[serde(default)]
    pub rent_payer: Option<String>,
    #[serde(default)]
    pub top_miner: Option<String>,
    #[serde(default)]
    pub top_miner_reward: Option<u64>,
    #[serde(default)]
    pub total_deployed: Option<u64>,
    #[serde(default)]
    pub total_miners: Option<u64>,
    #[serde(default)]
    pub total_vaulted: Option<u64>,
    #[serde(default)]
    pub total_winnings: Option<u64>,
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

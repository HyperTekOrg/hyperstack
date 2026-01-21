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
pub struct Round {
    #[serde(rename = "id", default)]
    pub id: Option<u64>,
    #[serde(rename = "deployed", default)]
    pub deployed: Option<Vec<u64>>,
    #[serde(rename = "slotHash", default)]
    pub slot_hash: Option<Vec<i64>>,
    #[serde(rename = "count", default)]
    pub count: Option<Vec<u64>>,
    #[serde(rename = "expiresAt", default)]
    pub expires_at: Option<u64>,
    #[serde(rename = "motherlode", default)]
    pub motherlode: Option<u64>,
    #[serde(rename = "rentPayer", default)]
    pub rent_payer: Option<String>,
    #[serde(rename = "topMiner", default)]
    pub top_miner: Option<String>,
    #[serde(rename = "topMinerReward", default)]
    pub top_miner_reward: Option<u64>,
    #[serde(rename = "totalDeployed", default)]
    pub total_deployed: Option<u64>,
    #[serde(rename = "totalMiners", default)]
    pub total_miners: Option<u64>,
    #[serde(rename = "totalVaulted", default)]
    pub total_vaulted: Option<u64>,
    #[serde(rename = "totalWinnings", default)]
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

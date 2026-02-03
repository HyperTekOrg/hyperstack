use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenId {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub bonding_curve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenInfo {
    #[serde(default)]
    pub name: Option<Option<String>>,
    #[serde(default)]
    pub symbol: Option<Option<String>>,
    #[serde(default)]
    pub uri: Option<Option<String>>,
    #[serde(default)]
    pub is_complete: Option<Option<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenReserves {
    #[serde(default)]
    pub virtual_token_reserves: Option<Option<u64>>,
    #[serde(default)]
    pub virtual_sol_reserves: Option<Option<u64>>,
    #[serde(default)]
    pub real_token_reserves: Option<Option<u64>>,
    #[serde(default)]
    pub real_sol_reserves: Option<Option<u64>>,
    #[serde(default)]
    pub token_total_supply: Option<Option<u64>>,
    #[serde(default)]
    pub current_price_sol: Option<Option<f64>>,
    #[serde(default)]
    pub market_cap_sol: Option<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenTrading {
    #[serde(default)]
    pub total_buy_volume: Option<Option<u64>>,
    #[serde(default)]
    pub total_sell_volume: Option<Option<u64>>,
    #[serde(default)]
    pub total_buy_exact_sol_volume: Option<Option<u64>>,
    #[serde(default)]
    pub total_trades: Option<Option<u64>>,
    #[serde(default)]
    pub buy_count: Option<Option<u64>>,
    #[serde(default)]
    pub sell_count: Option<Option<u64>>,
    #[serde(default)]
    pub unique_traders: Option<Option<u64>>,
    #[serde(default)]
    pub largest_trade: Option<Option<u64>>,
    #[serde(default)]
    pub smallest_trade: Option<Option<u64>>,
    #[serde(default)]
    pub last_trade_timestamp: Option<Option<i64>>,
    #[serde(default)]
    pub last_trade_price: Option<Option<f64>>,
    #[serde(default)]
    pub whale_trade_count: Option<Option<u64>>,
    #[serde(default)]
    pub last_whale_address: Option<Option<String>>,
    #[serde(default)]
    pub total_volume: Option<Option<u64>>,
    #[serde(default)]
    pub average_trade_size: Option<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenEvents {
    #[serde(default)]
    pub create: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub create_v2: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub buys: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub buys_exact_sol: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub sells: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunToken {
    #[serde(default)]
    pub id: PumpfunTokenId,
    #[serde(default)]
    pub info: PumpfunTokenInfo,
    #[serde(default)]
    pub reserves: PumpfunTokenReserves,
    #[serde(default)]
    pub trading: PumpfunTokenTrading,
    #[serde(default)]
    pub events: PumpfunTokenEvents,
    #[serde(default)]
    pub bonding_curve_snapshot: Option<Option<serde_json::Value>>,
}



#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Create {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub mint_authority: Option<String>,
    #[serde(default)]
    pub bonding_curve: Option<String>,
    #[serde(default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(default)]
    pub global: Option<String>,
    #[serde(default)]
    pub mpl_token_metadata: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub system_program: Option<String>,
    #[serde(default)]
    pub token_program: Option<String>,
    #[serde(default)]
    pub associated_token_program: Option<String>,
    #[serde(default)]
    pub rent: Option<String>,
    #[serde(default)]
    pub event_authority: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Buy {
    #[serde(default)]
    pub global: Option<String>,
    #[serde(default)]
    pub fee_recipient: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub bonding_curve: Option<String>,
    #[serde(default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(default)]
    pub associated_user: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub system_program: Option<String>,
    #[serde(default)]
    pub token_program: Option<String>,
    #[serde(default)]
    pub creator_vault: Option<String>,
    #[serde(default)]
    pub event_authority: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub global_volume_accumulator: Option<String>,
    #[serde(default)]
    pub user_volume_accumulator: Option<String>,
    #[serde(default)]
    pub fee_config: Option<String>,
    #[serde(default)]
    pub fee_program: Option<String>,
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default)]
    pub max_sol_cost: Option<u64>,
    #[serde(default)]
    pub track_volume: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sell {
    #[serde(default)]
    pub global: Option<String>,
    #[serde(default)]
    pub fee_recipient: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default)]
    pub bonding_curve: Option<String>,
    #[serde(default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(default)]
    pub associated_user: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub system_program: Option<String>,
    #[serde(default)]
    pub creator_vault: Option<String>,
    #[serde(default)]
    pub token_program: Option<String>,
    #[serde(default)]
    pub event_authority: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub fee_config: Option<String>,
    #[serde(default)]
    pub fee_program: Option<String>,
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default)]
    pub min_sol_output: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BondingCurve {
    #[serde(default)]
    pub virtual_token_reserves: Option<u64>,
    #[serde(default)]
    pub virtual_sol_reserves: Option<u64>,
    #[serde(default)]
    pub real_token_reserves: Option<u64>,
    #[serde(default)]
    pub real_sol_reserves: Option<u64>,
    #[serde(default)]
    pub token_total_supply: Option<u64>,
    #[serde(default)]
    pub complete: Option<bool>,
    #[serde(default)]
    pub creator: Option<String>,
    #[serde(default)]
    pub is_mayhem_mode: Option<bool>,
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

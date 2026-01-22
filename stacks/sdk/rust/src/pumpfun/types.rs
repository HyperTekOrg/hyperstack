use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenId {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(rename = "bondingCurve", default)]
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
    #[serde(rename = "isComplete", default)]
    pub is_complete: Option<Option<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenReserves {
    #[serde(rename = "virtualTokenReserves", default)]
    pub virtual_token_reserves: Option<Option<u64>>,
    #[serde(rename = "virtualSolReserves", default)]
    pub virtual_sol_reserves: Option<Option<u64>>,
    #[serde(rename = "realTokenReserves", default)]
    pub real_token_reserves: Option<Option<u64>>,
    #[serde(rename = "realSolReserves", default)]
    pub real_sol_reserves: Option<Option<u64>>,
    #[serde(rename = "tokenTotalSupply", default)]
    pub token_total_supply: Option<Option<u64>>,
    #[serde(rename = "currentPriceSol", default)]
    pub current_price_sol: Option<Option<f64>>,
    #[serde(rename = "marketCapSol", default)]
    pub market_cap_sol: Option<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenTrading {
    #[serde(rename = "totalBuyVolume", default)]
    pub total_buy_volume: Option<Option<u64>>,
    #[serde(rename = "totalSellVolume", default)]
    pub total_sell_volume: Option<Option<u64>>,
    #[serde(rename = "totalBuyExactSolVolume", default)]
    pub total_buy_exact_sol_volume: Option<Option<u64>>,
    #[serde(rename = "totalTrades", default)]
    pub total_trades: Option<Option<u64>>,
    #[serde(rename = "buyCount", default)]
    pub buy_count: Option<Option<u64>>,
    #[serde(rename = "sellCount", default)]
    pub sell_count: Option<Option<u64>>,
    #[serde(rename = "uniqueTraders", default)]
    pub unique_traders: Option<Option<u64>>,
    #[serde(rename = "largestTrade", default)]
    pub largest_trade: Option<Option<u64>>,
    #[serde(rename = "smallestTrade", default)]
    pub smallest_trade: Option<Option<u64>>,
    #[serde(rename = "lastTradeTimestamp", default)]
    pub last_trade_timestamp: Option<Option<i64>>,
    #[serde(rename = "lastTradePrice", default)]
    pub last_trade_price: Option<Option<f64>>,
    #[serde(rename = "whaleTradeCount", default)]
    pub whale_trade_count: Option<Option<u64>>,
    #[serde(rename = "lastWhaleAddress", default)]
    pub last_whale_address: Option<Option<String>>,
    #[serde(rename = "totalVolume", default)]
    pub total_volume: Option<Option<u64>>,
    #[serde(rename = "averageTradeSize", default)]
    pub average_trade_size: Option<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenEvents {
    #[serde(default)]
    pub create: Option<Option<serde_json::Value>>,
    #[serde(rename = "createV2", default)]
    pub create_v2: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub buys: Option<Vec<serde_json::Value>>,
    #[serde(rename = "buysExactSol", default)]
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
    #[serde(rename = "bondingCurveSnapshot", default)]
    pub bonding_curve_snapshot: Option<Option<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Create {
    #[serde(rename = "mint", default)]
    pub mint: Option<String>,
    #[serde(rename = "mintAuthority", default)]
    pub mint_authority: Option<String>,
    #[serde(rename = "bondingCurve", default)]
    pub bonding_curve: Option<String>,
    #[serde(rename = "associatedBondingCurve", default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(rename = "global", default)]
    pub global: Option<String>,
    #[serde(rename = "mplTokenMetadata", default)]
    pub mpl_token_metadata: Option<String>,
    #[serde(rename = "metadata", default)]
    pub metadata: Option<String>,
    #[serde(rename = "user", default)]
    pub user: Option<String>,
    #[serde(rename = "systemProgram", default)]
    pub system_program: Option<String>,
    #[serde(rename = "tokenProgram", default)]
    pub token_program: Option<String>,
    #[serde(rename = "associatedTokenProgram", default)]
    pub associated_token_program: Option<String>,
    #[serde(rename = "rent", default)]
    pub rent: Option<String>,
    #[serde(rename = "eventAuthority", default)]
    pub event_authority: Option<String>,
    #[serde(rename = "program", default)]
    pub program: Option<String>,
    #[serde(rename = "name", default)]
    pub name: Option<String>,
    #[serde(rename = "symbol", default)]
    pub symbol: Option<String>,
    #[serde(rename = "uri", default)]
    pub uri: Option<String>,
    #[serde(rename = "creator", default)]
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Buy {
    #[serde(rename = "global", default)]
    pub global: Option<String>,
    #[serde(rename = "feeRecipient", default)]
    pub fee_recipient: Option<String>,
    #[serde(rename = "mint", default)]
    pub mint: Option<String>,
    #[serde(rename = "bondingCurve", default)]
    pub bonding_curve: Option<String>,
    #[serde(rename = "associatedBondingCurve", default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(rename = "associatedUser", default)]
    pub associated_user: Option<String>,
    #[serde(rename = "user", default)]
    pub user: Option<String>,
    #[serde(rename = "systemProgram", default)]
    pub system_program: Option<String>,
    #[serde(rename = "tokenProgram", default)]
    pub token_program: Option<String>,
    #[serde(rename = "creatorVault", default)]
    pub creator_vault: Option<String>,
    #[serde(rename = "eventAuthority", default)]
    pub event_authority: Option<String>,
    #[serde(rename = "program", default)]
    pub program: Option<String>,
    #[serde(rename = "globalVolumeAccumulator", default)]
    pub global_volume_accumulator: Option<String>,
    #[serde(rename = "userVolumeAccumulator", default)]
    pub user_volume_accumulator: Option<String>,
    #[serde(rename = "feeConfig", default)]
    pub fee_config: Option<String>,
    #[serde(rename = "feeProgram", default)]
    pub fee_program: Option<String>,
    #[serde(rename = "amount", default)]
    pub amount: Option<u64>,
    #[serde(rename = "maxSolCost", default)]
    pub max_sol_cost: Option<u64>,
    #[serde(rename = "trackVolume", default)]
    pub track_volume: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sell {
    #[serde(rename = "global", default)]
    pub global: Option<String>,
    #[serde(rename = "feeRecipient", default)]
    pub fee_recipient: Option<String>,
    #[serde(rename = "mint", default)]
    pub mint: Option<String>,
    #[serde(rename = "bondingCurve", default)]
    pub bonding_curve: Option<String>,
    #[serde(rename = "associatedBondingCurve", default)]
    pub associated_bonding_curve: Option<String>,
    #[serde(rename = "associatedUser", default)]
    pub associated_user: Option<String>,
    #[serde(rename = "user", default)]
    pub user: Option<String>,
    #[serde(rename = "systemProgram", default)]
    pub system_program: Option<String>,
    #[serde(rename = "creatorVault", default)]
    pub creator_vault: Option<String>,
    #[serde(rename = "tokenProgram", default)]
    pub token_program: Option<String>,
    #[serde(rename = "eventAuthority", default)]
    pub event_authority: Option<String>,
    #[serde(rename = "program", default)]
    pub program: Option<String>,
    #[serde(rename = "feeConfig", default)]
    pub fee_config: Option<String>,
    #[serde(rename = "feeProgram", default)]
    pub fee_program: Option<String>,
    #[serde(rename = "amount", default)]
    pub amount: Option<u64>,
    #[serde(rename = "minSolOutput", default)]
    pub min_sol_output: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BondingCurve {
    #[serde(rename = "virtualTokenReserves", default)]
    pub virtual_token_reserves: Option<u64>,
    #[serde(rename = "virtualSolReserves", default)]
    pub virtual_sol_reserves: Option<u64>,
    #[serde(rename = "realTokenReserves", default)]
    pub real_token_reserves: Option<u64>,
    #[serde(rename = "realSolReserves", default)]
    pub real_sol_reserves: Option<u64>,
    #[serde(rename = "tokenTotalSupply", default)]
    pub token_total_supply: Option<u64>,
    #[serde(rename = "complete", default)]
    pub complete: Option<bool>,
    #[serde(rename = "creator", default)]
    pub creator: Option<String>,
    #[serde(rename = "isMayhemMode", default)]
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

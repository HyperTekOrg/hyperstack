use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenId {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default, rename = "bondingCurve")]
    pub bonding_curve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenInfo {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default, rename = "isComplete")]
    pub is_complete: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenReserves {
    #[serde(default, rename = "virtualTokenReserves")]
    pub virtual_token_reserves: Option<u64>,
    #[serde(default, rename = "virtualSolReserves")]
    pub virtual_sol_reserves: Option<u64>,
    #[serde(default, rename = "realTokenReserves")]
    pub real_token_reserves: Option<u64>,
    #[serde(default, rename = "realSolReserves")]
    pub real_sol_reserves: Option<u64>,
    #[serde(default, rename = "tokenTotalSupply")]
    pub token_total_supply: Option<u64>,
    #[serde(default, rename = "currentPriceSol")]
    pub current_price_sol: Option<f64>,
    #[serde(default, rename = "marketCapSol")]
    pub market_cap_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenTrading {
    #[serde(default, rename = "totalBuyVolume")]
    pub total_buy_volume: Option<u64>,
    #[serde(default, rename = "totalSellVolume")]
    pub total_sell_volume: Option<u64>,
    #[serde(default, rename = "totalTrades")]
    pub total_trades: Option<u64>,
    #[serde(default, rename = "buyCount")]
    pub buy_count: Option<u64>,
    #[serde(default, rename = "sellCount")]
    pub sell_count: Option<u64>,
    #[serde(default, rename = "uniqueTraders")]
    pub unique_traders: Option<u64>,
    #[serde(default, rename = "largestTrade")]
    pub largest_trade: Option<u64>,
    #[serde(default, rename = "smallestTrade")]
    pub smallest_trade: Option<u64>,
    #[serde(default, rename = "lastTradeTimestamp")]
    pub last_trade_timestamp: Option<i64>,
    #[serde(default, rename = "lastTradePrice")]
    pub last_trade_price: Option<f64>,
    #[serde(default, rename = "whaleTradeCount")]
    pub whale_trade_count: Option<u64>,
    #[serde(default, rename = "lastWhaleAddress")]
    pub last_whale_address: Option<String>,
    #[serde(default, rename = "totalVolume")]
    pub total_volume: Option<u64>,
    #[serde(default, rename = "averageTradeSize")]
    pub average_trade_size: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PumpfunTokenEvents {
    #[serde(default)]
    pub create: Option<Create>,
    #[serde(default)]
    pub buys: Option<Vec<EventWrapper<Buy>>>,
    #[serde(default)]
    pub sells: Option<Vec<EventWrapper<Sell>>>,
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
    #[serde(default, rename = "bondingCurveSnapshot")]
    pub bonding_curve_snapshot: Option<BondingCurve>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Create {
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default, rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    #[serde(default, rename = "bondingCurve")]
    pub bonding_curve: Option<String>,
    #[serde(default, rename = "associatedBondingCurve")]
    pub associated_bonding_curve: Option<String>,
    #[serde(default)]
    pub global: Option<String>,
    #[serde(default, rename = "mplTokenMetadata")]
    pub mpl_token_metadata: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default, rename = "systemProgram")]
    pub system_program: Option<String>,
    #[serde(default, rename = "tokenProgram")]
    pub token_program: Option<String>,
    #[serde(default, rename = "associatedTokenProgram")]
    pub associated_token_program: Option<String>,
    #[serde(default)]
    pub rent: Option<String>,
    #[serde(default, rename = "eventAuthority")]
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
    #[serde(default, rename = "feeRecipient")]
    pub fee_recipient: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default, rename = "bondingCurve")]
    pub bonding_curve: Option<String>,
    #[serde(default, rename = "associatedBondingCurve")]
    pub associated_bonding_curve: Option<String>,
    #[serde(default, rename = "associatedUser")]
    pub associated_user: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default, rename = "systemProgram")]
    pub system_program: Option<String>,
    #[serde(default, rename = "tokenProgram")]
    pub token_program: Option<String>,
    #[serde(default, rename = "creatorVault")]
    pub creator_vault: Option<String>,
    #[serde(default, rename = "eventAuthority")]
    pub event_authority: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default, rename = "maxSolCost")]
    pub max_sol_cost: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Sell {
    #[serde(default)]
    pub global: Option<String>,
    #[serde(default, rename = "feeRecipient")]
    pub fee_recipient: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    #[serde(default, rename = "bondingCurve")]
    pub bonding_curve: Option<String>,
    #[serde(default, rename = "associatedBondingCurve")]
    pub associated_bonding_curve: Option<String>,
    #[serde(default, rename = "associatedUser")]
    pub associated_user: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default, rename = "systemProgram")]
    pub system_program: Option<String>,
    #[serde(default, rename = "creatorVault")]
    pub creator_vault: Option<String>,
    #[serde(default, rename = "tokenProgram")]
    pub token_program: Option<String>,
    #[serde(default, rename = "eventAuthority")]
    pub event_authority: Option<String>,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub amount: Option<u64>,
    #[serde(default, rename = "minSolOutput")]
    pub min_sol_output: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BondingCurve {
    #[serde(default, rename = "virtualTokenReserves")]
    pub virtual_token_reserves: Option<u64>,
    #[serde(default, rename = "virtualSolReserves")]
    pub virtual_sol_reserves: Option<u64>,
    #[serde(default, rename = "realTokenReserves")]
    pub real_token_reserves: Option<u64>,
    #[serde(default, rename = "realSolReserves")]
    pub real_sol_reserves: Option<u64>,
    #[serde(default, rename = "tokenTotalSupply")]
    pub token_total_supply: Option<u64>,
    #[serde(default)]
    pub complete: Option<bool>,
    #[serde(default)]
    pub creator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWrapper<T> {
    #[serde(default)]
    pub timestamp: i64,
    pub data: T,
    #[serde(default)]
    pub slot: Option<u64>,
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

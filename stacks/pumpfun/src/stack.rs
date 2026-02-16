use hyperstack::macros::hyperstack;

#[hyperstack(idl = "idl/pump.json")]
pub mod pumpfun_stream {
    use hyperstack::macros::Stream;
    use serde::{Deserialize, Serialize};

    // Entity definition using IDL-generated types
    #[entity(name = "PumpfunToken")]
    pub struct PumpfunToken {
        pub id: TokenId,
        pub info: TokenInfo,
        pub reserves: ReserveState,
        pub trading: TradingMetrics,
        pub events: TokenEvents,

        // Snapshot the entire bonding curve account state with field transformations
        // The resolver (resolve_bonding_curve_key) handles the lookup from bonding_curve address to mint
        //
        // Field Transformations:
        // - creator: HexEncode - Converts the 32-byte pubkey array to a readable hex string
        //
        // Available Transformations:
        // - HexEncode: Converts byte arrays to hex strings
        // - HexDecode: Converts hex strings to byte arrays
        // - ToString: Converts any value to string representation
        // - ToNumber: Converts string representations to numbers
        #[snapshot(strategy = LastWrite, transforms = [(creator, HexEncode)])]
        pub bonding_curve_snapshot: Option<pump_sdk::accounts::BondingCurve>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TokenId {
        #[from_instruction([pump_sdk::instructions::Create::mint, pump_sdk::instructions::CreateV2::mint], primary_key, strategy = SetOnce)]
        pub mint: String,

        #[map(pump_sdk::accounts::BondingCurve::__account_address, lookup_index(
            register_from = [
                (pump_sdk::instructions::Create, accounts::bonding_curve, accounts::mint),
                (pump_sdk::instructions::CreateV2, accounts::bonding_curve, accounts::mint),
                (pump_sdk::instructions::Buy, accounts::bonding_curve, accounts::mint),
                (pump_sdk::instructions::BuyExactSolIn, accounts::bonding_curve, accounts::mint),
                (pump_sdk::instructions::Sell, accounts::bonding_curve, accounts::mint)
            ]
        ), strategy = SetOnce)]
        pub bonding_curve: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TokenInfo {
        #[from_instruction([pump_sdk::instructions::Create::name, pump_sdk::instructions::CreateV2::name], strategy = SetOnce)]
        pub name: Option<String>,

        #[from_instruction([pump_sdk::instructions::Create::symbol, pump_sdk::instructions::CreateV2::symbol], strategy = SetOnce)]
        pub symbol: Option<String>,

        #[from_instruction([pump_sdk::instructions::Create::uri, pump_sdk::instructions::CreateV2::uri], strategy = SetOnce)]
        pub uri: Option<String>,

        #[map(pump_sdk::accounts::BondingCurve::complete, strategy = LastWrite)]
        pub is_complete: Option<bool>,

        // URL resolver: fetch and extract image from metadata URI
        #[resolve(url = info.uri, extract = "image")]
        pub resolved_image: Option<String>,
    }

    // ReserveState section: All fields come from BondingCurve account updates
    // The resolver system (see resolve_bonding_curve_key below) handles mapping
    // the BondingCurve PDA address back to the mint primary key
    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct ReserveState {
        #[map(pump_sdk::accounts::BondingCurve::virtual_token_reserves, strategy = LastWrite)]
        pub virtual_token_reserves: Option<u64>,

        #[map(pump_sdk::accounts::BondingCurve::virtual_sol_reserves, strategy = LastWrite)]
        pub virtual_sol_reserves: Option<u64>,

        #[map(pump_sdk::accounts::BondingCurve::real_token_reserves, strategy = LastWrite)]
        pub real_token_reserves: Option<u64>,

        #[map(pump_sdk::accounts::BondingCurve::real_sol_reserves, strategy = LastWrite)]
        pub real_sol_reserves: Option<u64>,

        #[map(pump_sdk::accounts::BondingCurve::token_total_supply, strategy = LastWrite)]
        pub token_total_supply: Option<u64>,

        pub current_price_sol: Option<f64>,
        pub market_cap_sol: Option<f64>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TradingMetrics {
        #[aggregate(from = pump_sdk::instructions::Buy, field = amount, strategy = Sum, lookup_by = accounts::mint)]
        pub total_buy_volume: Option<u64>,

        #[aggregate(from = pump_sdk::instructions::Sell, field = amount, strategy = Sum, lookup_by = accounts::mint)]
        pub total_sell_volume: Option<u64>,

        #[aggregate(from = pump_sdk::instructions::BuyExactSolIn, field = spendable_sol_in, strategy = Sum, lookup_by = accounts::mint)]
        pub total_buy_exact_sol_volume: Option<u64>,

        #[aggregate(from = [pump_sdk::instructions::Buy, pump_sdk::instructions::BuyExactSolIn, pump_sdk::instructions::Sell], strategy = Count, lookup_by = accounts::mint)]
        pub total_trades: Option<u64>,

        #[aggregate(from = [pump_sdk::instructions::Buy, pump_sdk::instructions::BuyExactSolIn], strategy = Count, lookup_by = accounts::mint)]
        pub buy_count: Option<u64>,

        #[aggregate(from = pump_sdk::instructions::Sell, strategy = Count, lookup_by = accounts::mint)]
        pub sell_count: Option<u64>,

        #[aggregate(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::BuyExactSolIn, pump_sdk::instructions::Sell],
            field = user,
            strategy = UniqueCount,
            transform = ToString,
            lookup_by = accounts::mint
        )]
        pub unique_traders: Option<u64>,

        #[aggregate(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::Sell],
            field = amount,
            strategy = Max,
            lookup_by = accounts::mint
        )]
        pub largest_trade: Option<u64>,

        #[aggregate(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::Sell],
            field = amount,
            strategy = Min,
            lookup_by = accounts::mint
        )]
        pub smallest_trade: Option<u64>,

        #[derive_from(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::BuyExactSolIn, pump_sdk::instructions::Sell],
            field = __timestamp,
            lookup_by = accounts::mint
        )]
        pub last_trade_timestamp: Option<i64>,

        #[computed((reserves.virtual_sol_reserves.unwrap_or(0) as f64) / (reserves.virtual_token_reserves.unwrap_or(1).max(1) as f64))]
        pub last_trade_price: Option<f64>,

        #[aggregate(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::Sell],
            field = amount,
            strategy = Count,
            lookup_by = accounts::mint,
            condition = "data.amount > 1_000_000_000_000"
        )]
        pub whale_trade_count: Option<u64>,

        #[derive_from(
            from = [pump_sdk::instructions::Buy, pump_sdk::instructions::Sell],
            field = accounts::user,
            lookup_by = accounts::mint,
            condition = "data.amount > 1_000_000_000_000"
        )]
        pub last_whale_address: Option<String>,

        #[computed(total_buy_volume.unwrap_or(0) + total_sell_volume.unwrap_or(0))]
        pub total_volume: Option<u64>,

        #[computed(total_volume.unwrap_or(0) as f64 / total_trades.unwrap_or(1).max(1) as f64)]
        pub average_trade_size: Option<f64>,
    }

    //
    // ========================================================================
    // Token Events Section - Demonstrates New Type-Safe Event Macro
    // ========================================================================
    //
    // IMPORTANT: Event Runtime Types
    // -------------------------------
    // Events are wrapped in `transform::EventWrapper<T>` at runtime, which includes:
    // - `timestamp`: i64       - Unix timestamp when the event was processed
    // - `data`: T              - The actual event data (instruction or filtered fields)
    // - `slot`: Option<u64>    - Blockchain slot number (from UpdateContext)
    // - `signature`: Option<String> - Transaction signature (from UpdateContext)
    //
    // For example:
    // - `pub buys: Vec<pump_sdk::instructions::Buy>` in spec
    // - Runtime type: `Vec<EventWrapper<{ amount, max_sol_cost, user }>>`
    // - Output: `[{ timestamp: 123, data: { amount: 100, ... }, slot: 456, signature: "..." }]`
    //
    // This ensures:
    // 1. Type-safe event data based on your spec
    // 2. Consistent metadata (timestamp, slot, signature) for all events
    // 3. Clean separation between event data and context
    //
    // ========================================================================
    // Token Snapshots Section - Demonstrates New Type-Safe Snapshot Macro
    // ========================================================================
    //
    // IMPORTANT: Snapshot Macro for Account Snapshots
    // -----------------------------------------------
    // The #[snapshot] macro allows capturing entire account states at a point in time.
    // Unlike #[event] (for instructions), #[snapshot] is specifically for accounts.
    //
    // Key differences from #[event]:
    // - No `Append` strategy: Only `SetOnce` or `LastWrite` allowed
    // - Captures the entire account, not individual fields
    // - Used for account snapshots, not instruction events
    // - Supports field-level transformations (e.g., HexEncode for pubkeys)
    //
    // Snapshotted Data:
    // - Automatically filters out internal metadata fields (__account_address, __update_context, etc.)
    // - Applies field transformations specified in the macro
    // - Can be wrapped in transform::SnapshotWrapper for consistent metadata (timestamp, slot, signature)
    //
    // Example Output (with transforms):
    // {
    //   "bonding_curve_snapshot": {
    //     "complete": false,
    //     "creator": "FpP8mKVXCJjrKEoZ9gGJ...",  // <- HexEncoded instead of raw bytes
    //     "real_sol_reserves": 38799829630,
    //     "real_token_reserves": 187979068742904,
    //     "token_total_supply": 1000000000000000,
    //     "virtual_sol_reserves": 68799829630,
    //     "virtual_token_reserves": 467879068742904
    //   }
    // }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TokenEvents {
        #[event(strategy = SetOnce, lookup_by = accounts::mint)]
        pub create: Option<pump_sdk::instructions::Create>,

        #[event(strategy = SetOnce, lookup_by = accounts::mint)]
        pub create_v2: Option<pump_sdk::instructions::CreateV2>,

        #[event(
            strategy = Append,
            lookup_by = accounts::mint,
            fields = [data::amount, data::max_sol_cost, accounts::user]
        )]
        pub buys: Vec<pump_sdk::instructions::Buy>,

        #[event(
            strategy = Append,
            lookup_by = accounts::mint,
            fields = [data::spendable_sol_in, data::min_tokens_out, accounts::user]
        )]
        pub buys_exact_sol: Vec<pump_sdk::instructions::BuyExactSolIn>,

        #[event(
            strategy = Append,
            lookup_by = accounts::mint
        )]
        pub sells: Vec<pump_sdk::instructions::Sell>,
    }
}

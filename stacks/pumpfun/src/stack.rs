use hyperstack_macros::hyperstack;

#[hyperstack(idl = "idl/pump.json")]
pub mod pumpfun_stream {
    use hyperstack_macros::Stream;
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
        pub bonding_curve_snapshot: Option<generated_sdk::accounts::BondingCurve>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TokenId {
        #[from_instruction(generated_sdk::instructions::Create::mint, primary_key, strategy = SetOnce)]
        pub mint: String,

        #[from_instruction(generated_sdk::instructions::Create::bonding_curve, lookup_index, strategy = SetOnce)]
        pub bonding_curve: String,
    }

    // TokenInfo section: Maps data from instructions (Create) and accounts (BondingCurve)
    // - name, symbol, uri: Captured once from the Create instruction
    // - is_complete: Updated whenever the BondingCurve account changes via the resolver system
    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TokenInfo {
        #[from_instruction(generated_sdk::instructions::Create::name, strategy = SetOnce)]
        pub name: Option<String>,

        #[from_instruction(generated_sdk::instructions::Create::symbol, strategy = SetOnce)]
        pub symbol: Option<String>,

        #[from_instruction(generated_sdk::instructions::Create::uri, strategy = SetOnce)]
        pub uri: Option<String>,
        // Uses resolver to map BondingCurve account updates to the mint primary key
        #[map(generated_sdk::accounts::BondingCurve::complete, strategy = LastWrite)]
        pub is_complete: Option<bool>,
    }

    // ReserveState section: All fields come from BondingCurve account updates
    // The resolver system (see resolve_bonding_curve_key below) handles mapping
    // the BondingCurve PDA address back to the mint primary key
    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct ReserveState {
        #[map(generated_sdk::accounts::BondingCurve::virtual_token_reserves, strategy = LastWrite)]
        pub virtual_token_reserves: Option<u64>,

        #[map(generated_sdk::accounts::BondingCurve::virtual_sol_reserves, strategy = LastWrite)]
        pub virtual_sol_reserves: Option<u64>,

        #[map(generated_sdk::accounts::BondingCurve::real_token_reserves, strategy = LastWrite)]
        pub real_token_reserves: Option<u64>,

        #[map(generated_sdk::accounts::BondingCurve::real_sol_reserves, strategy = LastWrite)]
        pub real_sol_reserves: Option<u64>,

        #[map(generated_sdk::accounts::BondingCurve::token_total_supply, strategy = LastWrite)]
        pub token_total_supply: Option<u64>,

        pub current_price_sol: Option<f64>,
        pub market_cap_sol: Option<f64>,
    }

    // ========================================================================
    // Trading Metrics Section - Demonstrates Declarative Aggregations
    // ========================================================================
    //
    // The #[aggregate] macro provides a declarative way to accumulate metrics
    // across multiple Buy/Sell events. Each field specifies:
    // - from: Which instruction type(s) to aggregate from
    // - field: Which field to aggregate (optional for Count strategy)
    // - strategy: How to aggregate (Sum, Count, Min, Max, UniqueCount)
    // - condition: Optional condition expression for conditional aggregation (Level 1!)
    //
    // Available Strategies:
    // - Sum: Accumulate numeric values
    // - Count: Count occurrences (increments by 1)
    // - Min: Track minimum value
    // - Max: Track maximum value
    // - UniqueCount: Count unique values (maintains internal Set)
    //
    // The VM automatically generates opcodes to:
    // 1. Initialize fields to 0/null on first update
    // 2. Apply aggregation strategy on each event
    // 3. Maintain internal state (e.g., Sets for UniqueCount)
    //
    #[derive(Debug, Clone, Serialize, Deserialize, Stream)]
    pub struct TradingMetrics {
        // ========================================================================
        // Declarative Aggregations - Using #[aggregate] Macro
        // ========================================================================

        // Sum aggregations for trade volumes
        #[aggregate(from = generated_sdk::instructions::Buy, field = amount, strategy = Sum, lookup_by = accounts::mint)]
        pub total_buy_volume: Option<u64>,

        #[aggregate(from = generated_sdk::instructions::Sell, field = amount, strategy = Sum, lookup_by = accounts::mint)]
        pub total_sell_volume: Option<u64>,

        // Count aggregations for trade occurrences
        #[aggregate(from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell], strategy = Count, lookup_by = accounts::mint)]
        pub total_trades: Option<u64>,

        #[aggregate(from = generated_sdk::instructions::Buy, strategy = Count, lookup_by = accounts::mint)]
        pub buy_count: Option<u64>,

        #[aggregate(from = generated_sdk::instructions::Sell, strategy = Count, lookup_by = accounts::mint)]
        pub sell_count: Option<u64>,

        // Unique trader counting (requires transform since user is a pubkey)
        #[aggregate(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = user,
            strategy = UniqueCount,
            transform = ToString,
            lookup_by = accounts::mint
        )]
        pub unique_traders: Option<u64>,

        // Min/Max aggregations for trade sizes
        #[aggregate(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = amount,
            strategy = Max,
            lookup_by = accounts::mint
        )]
        pub largest_trade: Option<u64>,

        #[aggregate(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = amount,
            strategy = Min,
            lookup_by = accounts::mint
        )]
        pub smallest_trade: Option<u64>,

        // Track timestamp from Buy/Sell instructions using special __timestamp field
        #[derive_from(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = __timestamp,
            lookup_by = accounts::mint
        )]
        pub last_trade_timestamp: Option<i64>,

        // Price calculation using cross-section computed field
        // References reserve state values from the ReserveState section
        // Formula: price = virtual_sol_reserves / virtual_token_reserves
        #[computed((reserves.virtual_sol_reserves.unwrap_or(0) as f64) / (reserves.virtual_token_reserves.unwrap_or(1).max(1) as f64))]
        pub last_trade_price: Option<f64>,

        // Count whale trades (amount > 1 trillion) using conditional aggregation
        #[aggregate(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = amount,
            strategy = Count,
            lookup_by = accounts::mint,
            condition = "data.amount > 1_000_000_000_000"
        )]
        pub whale_trade_count: Option<u64>,

        // Track last whale address using conditional field deriving
        #[derive_from(
            from = [generated_sdk::instructions::Buy, generated_sdk::instructions::Sell],
            field = accounts::user,
            lookup_by = accounts::mint,
            condition = "data.amount > 1_000_000_000_000"
        )]
        pub last_whale_address: Option<String>,

        // Computed/derived metrics (calculated from other fields)
        // These fields use #[computed] to automatically derive values from aggregated fields
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
    // - `pub buys: Vec<generated_sdk::instructions::Buy>` in spec
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
        // This snapshots the entire Create instruction while TokenId/TokenInfo map specific fields.
        #[event(strategy = SetOnce, lookup_by = accounts::mint)]
        pub create: Option<generated_sdk::instructions::Create>,

        // Runtime type: Vec<EventWrapper<{ amount, max_sol_cost, user }>>
        #[event(
            strategy = Append,
            lookup_by = accounts::mint,
            fields = [data::amount, data::max_sol_cost, accounts::user]
        )]
        pub buys: Vec<generated_sdk::instructions::Buy>,

        // Runtime type: Vec<EventWrapper<generated_sdk::instructions::Sell>>
        #[event(
            strategy = Append,
            lookup_by = accounts::mint
        )]
        pub sells: Vec<generated_sdk::instructions::Sell>,
    }

    // ========================================================================
    // PDA Resolution - Declarative (Level 1)
    // ========================================================================
    // Declarative syntax replaces imperative #[resolve_key_for] and #[after_instruction] hooks

    // Declare how to resolve BondingCurve account addresses to mint keys
    #[resolve_key(
        account = generated_sdk::accounts::BondingCurve,
        strategy = "pda_reverse_lookup",
        queue_until = [
            generated_sdk::instructions::Create,
            generated_sdk::instructions::Buy,
            generated_sdk::instructions::Sell
        ]
    )]
    struct BondingCurveResolver;

    // Register PDA mappings when each instruction is processed
    // Maps the bonding_curve PDA address to the mint primary key
    #[register_pda(
        instruction = generated_sdk::instructions::Create,
        pda_field = accounts::bonding_curve,
        primary_key = accounts::mint
    )]
    struct CreatePdaRegistration;

    #[register_pda(
        instruction = generated_sdk::instructions::Buy,
        pda_field = accounts::bonding_curve,
        primary_key = accounts::mint
    )]
    struct BuyPdaRegistration;

    #[register_pda(
        instruction = generated_sdk::instructions::Sell,
        pda_field = accounts::bonding_curve,
        primary_key = accounts::mint
    )]
    struct SellPdaRegistration;
}

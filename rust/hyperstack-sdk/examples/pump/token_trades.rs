use chrono::Utc;
use hyperstack_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TradeInfo {
    wallet: String,
    direction: String,
    amount_sol: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PumpToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    mint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_trade: Option<TradeInfo>,
}

struct PumpTokenEntity;

impl Entity for PumpTokenEntity {
    type Data = PumpToken;
    const NAME: &'static str = "PumpToken";

    fn state_view() -> &'static str {
        "PumpToken/state"
    }
    fn list_view() -> &'static str {
        "PumpToken/list"
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:8080".to_string());
    let mint = std::env::var("KEY").expect("KEY env var required (token mint address)");

    let hs = HyperStack::connect(&url).await?;

    println!("watching trades for token {}...\n", mint);

    let mut stream = hs.watch_key::<PumpTokenEntity>(&mint).await;
    let mut trade_history: VecDeque<TradeInfo> = VecDeque::with_capacity(100);

    loop {
        tokio::select! {
            Some(update) = stream.next() => {
                if let Update::Upsert { data: token, .. } | Update::Patch { data: token, .. } = update {
                    if let Some(trade) = token.last_trade {
                        trade_history.push_back(trade.clone());
                        if trade_history.len() > 100 {
                            trade_history.pop_front();
                        }

                        println!("[{}] {} | {}... | {:.4} SOL (total: {} trades)",
                            Utc::now().format("%H:%M:%S"),
                            trade.direction,
                            &trade.wallet[..8],
                            trade.amount_sol,
                            trade_history.len());
                    }
                }
            }
            _ = sleep(Duration::from_secs(60)) => {
                println!("No trades for 60s...");
            }
        }
    }
}

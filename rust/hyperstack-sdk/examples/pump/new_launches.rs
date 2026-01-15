use chrono::Utc;
use hyperstack_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

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
    virtual_sol_reserves: Option<u64>,
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

    let hs = HyperStack::connect(&url).await?;

    println!("watching for new pump.fun token launches...\n");

    let mut stream = hs.watch::<PumpTokenEntity>().await;

    loop {
        tokio::select! {
            Some(update) = stream.next() => {
                if let Update::Upsert { key: mint, data: token } = update {
                    if token.name.is_some() || token.symbol.is_some() {
                        println!("\n[{}] new token launch", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
                        println!("mint: {}", mint);
                        if let Some(name) = &token.name {
                            println!("name: {}", name);
                        }
                        if let Some(symbol) = &token.symbol {
                            println!("symbol: {}", symbol);
                        }
                        if let Some(creator) = &token.creator {
                            println!("creator: {}", creator);
                        }
                        if let Some(sol) = token.virtual_sol_reserves {
                            println!("initial liquidity: {:.4} SOL", sol as f64 / 1e9);
                        }
                    }
                }
            }
            _ = sleep(Duration::from_secs(60)) => {
                println!("No new launches for 60s...");
            }
        }
    }
}

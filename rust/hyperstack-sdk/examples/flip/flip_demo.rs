use hyperstack_sdk::HyperStackClient;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameId {
    global_count: Option<u64>,
    account_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameStatus {
    current: Option<String>,
    created_at: Option<i64>,
    activated_at: Option<i64>,
    settled_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameMetrics {
    total_volume: Option<i64>,
    total_ev: Option<i64>,
    bet_count: Option<i64>,
    unique_players: Option<i64>,
    total_fees_collected: Option<i64>,
    total_payouts_distributed: Option<i64>,
    house_profit_loss: Option<i64>,
    claim_rate: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameEvent {
    timestamp: i64,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct GameEvents {
    created: Option<GameEvent>,
    activated: Option<GameEvent>,
    #[serde(default)]
    bets_placed: Vec<GameEvent>,
    betting_closed: Option<GameEvent>,
    settled: Option<GameEvent>,
    #[serde(default)]
    payouts_claimed: Vec<GameEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct SettlementGame {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<GameId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<GameStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<GameMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    events: Option<GameEvents>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:8080".to_string());

    let view = std::env::var("VIEW").unwrap_or_else(|_| "SettlementGame/kv".to_string());

    let mut client = HyperStackClient::<SettlementGame>::new(url, &view);

    if let Ok(key) = std::env::var("KEY") {
        client = client.with_key(key);
    }

    let store = client.connect().await?;

    println!("connected, watching {}...\n", view);

    let mut updates = store.subscribe();

    loop {
        tokio::select! {
            Ok((key, game)) = updates.recv() => {
                println!("\n=== Game {} ===", key);
                println!("{}", serde_json::to_string_pretty(&game).unwrap_or_default());
            }
            _ = sleep(Duration::from_secs(30)) => {
                println!("No updates for 30s...");
            }
        }
    }
}

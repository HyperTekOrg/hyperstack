use hyperstack_sdk::prelude::*;
use hyperstack_stacks::ore::{OreMiner, OreRound, OreStack, OreTreasury};

fn print_round(prefix: &str, round: &OreRound) {
    println!("\n[{}] === Round Update ===", prefix);
    println!("[{}] Round ID: {:?}", prefix, round.id.round_id);
    println!("[{}] Address: {:?}", prefix, round.id.round_address);
    println!("[{}] Motherlode: {:?}", prefix, round.state.motherlode);
    println!("[{}] Total Deployed: {:?}", prefix, round.state.total_deployed);
    println!("[{}] Expires At: {:?}", prefix, round.state.expires_at);
    println!("[{}] Metrics: {:?}", prefix, round.metrics);
    println!();
}

fn print_treasury(prefix: &str, treasury: &OreTreasury) {
    println!("\n[{}] === Treasury Update ===", prefix);
    println!("[{}] Address: {:?}", prefix, treasury.id.address);
    println!("[{}] Balance: {:?}", prefix, treasury.state.balance);
    println!("[{}] Motherlode: {:?}", prefix, treasury.state.motherlode);
    println!("[{}] Total Refined: {:?}", prefix, treasury.state.total_refined);
    println!("[{}] Total Staked: {:?}", prefix, treasury.state.total_staked);
    println!("[{}] Total Unclaimed: {:?}", prefix, treasury.state.total_unclaimed);
    println!();
}

fn print_miner(prefix: &str, miner: &OreMiner) {
    println!("\n[{}] === Miner Update ===", prefix);
    println!("[{}] Authority: {:?}", prefix, miner.id.authority);
    println!("[{}] Miner Address: {:?}", prefix, miner.id.miner_address);
    println!("[{}] Round ID: {:?}", prefix, miner.state.round_id);
    println!("[{}] Rewards SOL: {:?}", prefix, miner.rewards.rewards_sol);
    println!("[{}] Rewards ORE: {:?}", prefix, miner.rewards.rewards_ore);
    println!(
        "[{}] Lifetime Deployed: {:?}",
        prefix, miner.rewards.lifetime_deployed
    );
    println!("[{}] Automation Executor: {:?}", prefix, miner.automation.executor);
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hs = HyperStack::<OreStack>::connect().await?;
    println!("Connected to ORE stack");

    // --- OreRound: stream the latest round ---
    let latest_view = hs.views.ore_round.latest();

    // --- OreTreasury: fetch the singleton treasury by address ---
    // Set ORE_TREASURY_ADDRESS env var, or pass a known treasury pubkey.
    let treasury_view = hs.views.ore_treasury.state();
    let treasury_address = std::env::var("ORE_TREASURY_ADDRESS")
        .expect("Set ORE_TREASURY_ADDRESS to the on-chain treasury account pubkey");

    // --- OreMiner: stream all miners from the list view ---
    let miner_list_view = hs.views.ore_miner.list();

    println!("=== Watching OreRound (latest), OreTreasury (state), OreMiner (list) ===\n");

    let round_handle = tokio::spawn(async move {
        let mut stream = latest_view.listen().take(1);
        while let Some(round) = stream.next().await {
            print_round("ROUND", &round);
        }
    });

    let treasury_handle = tokio::spawn(async move {
        let mut stream = treasury_view.listen(&treasury_address);
        if let Some(treasury) = stream.next().await {
            print_treasury("TREASURY", &treasury);
        }
    });

    let miner_handle = tokio::spawn(async move {
        let mut stream = miner_list_view.listen().take(3);
        while let Some(miner) = stream.next().await {
            print_miner("MINER", &miner);
        }
    });

    let _ = tokio::join!(round_handle, treasury_handle, miner_handle);
    Ok(())
}

use hyperstack_sdk::prelude::*;
use hyperstack_stacks::ore::{OreRound, OreStack};

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hs = HyperStack::<OreStack>::connect().await?;
    println!("Connected to ORE stack");

    let list_view = hs.views.ore_round.list();
    let latest_view = hs.views.ore_round.latest();

    println!("=== Watching List and Latest views ===");

    let list_handle = tokio::spawn(async move {
        let mut list_stream = list_view.listen();
        while let Some(round) = list_stream.next().await {
            print_round("LIST", &round);
        }
    });

    let mut latest_stream = latest_view.listen().take(1);
    while let Some(round) = latest_stream.next().await {
        print_round("LATEST", &round);
    }

    list_handle.await?;
    Ok(())
}

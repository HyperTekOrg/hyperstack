use hyperstack_sdk::prelude::*;
use hyperstack_stacks::ore::OreRoundViews;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hs = HyperStack::connect("wss://ore-round-ubhivg.stack.usehyperstack.com").await?;
    println!("Connected to wss://ore-round-ubhivg.stack.usehyperstack.com");

    let views = hs.views::<OreRoundViews>();

    let list_view = views.list();
    let latest_view = views.latest();

    println!("=== Watching List and Latest views ===");

    // Spawn a task to watch list view in parallel (all rounds)
    let list_handle = tokio::spawn(async move {
        let mut list_stream = list_view.watch();
        while let Some(update) = list_stream.next().await {
            match update {
                Update::Upsert { key, data } | Update::Patch { key, data } => {
                    println!("\n[LIST] === Round Update ===");
                    println!("[LIST] Key: {}", key);
                    println!("[LIST] Round ID: {:?}", data.id.round_id);
                    println!("[LIST] Address: {:?}", data.id.round_address);
                    println!("[LIST] Motherlode: {:?}", data.state.motherlode);
                    println!("[LIST] Total Deployed: {:?}", data.state.total_deployed);
                    println!("[LIST] Expires At: {:?}", data.state.expires_at);
                    println!("[LIST] Metrics: {:?}", data.metrics);
                    println!();
                }
                Update::Delete { key } => {
                    println!("[LIST] Round deleted: {}\n", key);
                }
            }
        }
    });

    let mut latest_stream = latest_view.watch().take(1);
    while let Some(update) = latest_stream.next().await {
        match update {
            Update::Upsert { key, data } | Update::Patch { key, data } => {
                println!("\n[LATEST] === Latest Round Update ===");
                println!("[LATEST] Key: {}", key);
                println!("[LATEST] Round ID: {:?}", data.id.round_id);
                println!("[LATEST] Address: {:?}", data.id.round_address);
                println!("[LATEST] Motherlode: {:?}", data.state.motherlode);
                println!("[LATEST] Total Deployed: {:?}", data.state.total_deployed);
                println!("[LATEST] Expires At: {:?}", data.state.expires_at);
                println!("[LATEST] Metrics: {:?}", data.metrics);
                println!();
            }
            Update::Delete { key } => {
                println!("[LATEST] Round deleted: {}\n", key);
            }
        }
    }

    list_handle.await?;
    Ok(())
}

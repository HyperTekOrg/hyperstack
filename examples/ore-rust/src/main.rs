use hyperstack_sdk::prelude::*;
use hyperstack_stacks::ore::OreRoundViews;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let hs = HyperStack::connect("ws://localhost:8878").await?;
    println!("Connected to ws://localhost:8878");

    let views = hs.views::<OreRoundViews>();

    // Test both list view AND latest view to compare
    let list_view = views.list();
    let latest_view = views.latest();

    println!("=== Testing LIST view ===");
    let mut list_stream = list_view.watch();

    // Spawn a task to watch latest view in parallel
    let latest_handle = tokio::spawn(async move {
        println!("=== Testing LATEST view ===");
        let mut latest_stream = latest_view.watch();
        while let Some(update) = latest_stream.next().await {
            match update {
                Update::Upsert { key, data } | Update::Patch { key, data } => {
                    println!("\n[LATEST] === Round Update ===");
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
    });

    // Watch list view in main task
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

    latest_handle.await?;
    Ok(())
}

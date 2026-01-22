use hyperstack_server::Server;
use pumpfun_stack as pumpfun_stream;
use std::net::SocketAddr;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let env_path = manifest_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path)?;
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let spec = pumpfun_stream::spec();

    println!("Starting PumpFun server on [::]:8877...");

    Server::builder()
        .spec(spec)
        .websocket()
        .bind("[::]:8877".parse::<SocketAddr>()?)
        .health_monitoring()
        .start()
        .await?;

    Ok(())
}

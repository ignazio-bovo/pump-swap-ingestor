use anyhow::Result;
use pump_swap_ingestor::db::{BackendDb};
use pump_swap_ingestor::wss_ingestor::WssIngestor;
use tokio::sync::mpsc;
use tokio::try_join;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let clickhouse_url = std::env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "http://localhost:8123".to_string());

    let db = BackendDb::new(&clickhouse_url);
    db.run_migrations().await;

    let mut ingestor = WssIngestor::new(
        "wss://api.mainnet-beta.solana.com",
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
    )
    .await?;

    let (tx, rx) = mpsc::unbounded_channel();

    let db_service_handle = tokio::task::spawn(async move {
        info!(" starting db service");
        db.trade_store_service(rx).await;
    });

    let ingestor_service_handle = tokio::task::spawn(async move {
        info!(" starting ingestor service");
        let _ = ingestor.ingest_trades(tx).await;
    });


    try_join!(db_service_handle, ingestor_service_handle)?;

    info!("✅ Both services completed");

    Ok(())
}

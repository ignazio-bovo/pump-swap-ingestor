use crate::trades::Trade;
use anyhow::Result;
use clickhouse::Client;
use std::fs;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info, warn};

pub struct BackendDb {
    db_client: Client,
}

impl BackendDb {
    pub fn new(url: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_user("bonk")
            // .with_password("")
            .with_database("pump_swap_data");

        Self { db_client: client }
    }

    pub async fn trade_store_service(&self, mut rx: UnboundedReceiver<Trade>) {
        while let Some(trade) = rx.recv().await {
            match self.upsert_trade(&trade).await {
                Ok(()) => {}
                Err(e) => {
                    error!("error inserting trade error {:?}", e.to_string())
                }
            }
        }
        warn!("📨 Trade receiver channel closed - no more trades will be processed");
    }

    // update or insert
    pub async fn upsert_trade(&self, trade: &Trade) -> Result<()> {
        let mut trades_handle = self.db_client.insert("trades")?;
        trades_handle.write(trade).await?;
        trades_handle.end().await?;
        Ok(())
    }

    pub async fn needs_migration(&self) -> Result<bool> {
        let query = "SELECT count() FROM system.tables WHERE database = currentDatabase() AND name = 'trades'";

        let result: u64 = self.db_client.query(query).fetch_one().await?;

        Ok(result == 0)
    }

    pub async fn run_migrations(&self) {
        let migration_sql =
            fs::read_to_string("migrations/001_trades.sql").expect("Migration file not found");

        let migration_should_be_run = self
            .needs_migration()
            .await
            .expect("Unable to establish if migrations are neeedd");
        if migration_should_be_run {
            self.db_client
                .query(&migration_sql)
                .execute()
                .await
                .expect("Migration setup is necessary for correct setup");

            info!("Migration completed successfully");
        }
    }

    pub fn get_trade(&self) {}
}

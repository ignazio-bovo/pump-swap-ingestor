use crate::trades::Trade;
use anyhow::Result;
use clickhouse::Client;
use clickhouse::sql::Bind;
use std::fs;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info, warn};

pub struct BackendDb {
    dbClient: Client,
}

impl BackendDb {
    pub fn new(url: &str) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_user("bonk")
            // .with_password("")
            .with_database("pump_swap_data");

        Self { dbClient: client }
    }

    pub async fn store_trades(&mut self, mut rx: UnboundedReceiver<Trade>) {
        while let Some(trade) = rx.recv().await {
            match self.upsert_trade(&trade).await {
                Ok(()) => {
                    info!("inserted trade at tx hash {:?} and log index {:?}", trade.tx_hash, trade.log_index)
                }
                Err(e) => {
                    error!("error inserting trade error {:?}", e.to_string())
                }
            }
        }
        warn!("📨 Trade receiver channel closed - no more trades will be processed");

    }

    // update or insert
    pub async fn upsert_trade(&self, trade: &Trade) -> Result<()> {
        let mut trades_handle = self.dbClient.insert("trades")?;
        trades_handle.write(trade).await?;
        trades_handle.end().await?;
        info!("✅ trade inserted into db tx hash {:?} log index {:?}", &trade.tx_hash, &trade.log_index);
        Ok(())
    }

    pub async fn needs_migration(&self) -> Result<bool> {
        let query = "SELECT count() FROM system.tables WHERE database = currentDatabase() AND name = 'trades'";

        let result: u64 = self.dbClient
            .query(query)
            .fetch_one()
            .await?;

        Ok(result == 0)
    }

    pub async fn run_migrations(&self) {
        let migration_sql =
            fs::read_to_string("migrations/001_trades.sql").expect("Migration file not found");

        let migration_should_be_run = self.needs_migration().await.expect("Unable to establish if migrations are neeedd");
        if migration_should_be_run {
            self.dbClient
                .query(&migration_sql)
                .execute()
                .await
                .expect("Migration setup is necessary for correct setup");

            info!("Migration completed successfully");
        }
    }

    pub fn get_trade(&self) {}
}

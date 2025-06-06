use crate::trades::Trade;
use anyhow::Result;
use clickhouse::Client;
use clickhouse::sql::Bind;
use std::fs;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info};

pub struct BackendDb {
    dbClient: Client,
    input_channel: UnboundedReceiver<Trade>
}

impl BackendDb {
    pub fn new(url: &str, rx: UnboundedReceiver<Trade>) -> Self {
        let client = Client::default()
            .with_url(url)
            .with_user("bonk")
            // .with_password("")
            .with_database("pump_swap_data");

        Self { dbClient: client, input_channel: rx }
    }

    pub async fn store_trades(&mut self) {
        while let Some(trade) = self.input_channel.recv().await {
            match self.upsert_trade(&trade).await {
                Ok(()) => {
                    error!("inserted trade at tx hash {:?} and log index {:?}", trade.tx_hash, trade.log_index)
                }
                Err(e) => {
                    error!("error inserting trade at tx hash {:?} and log index {:?}", trade.tx_hash, trade.log_index)
                }
            }

        }

    }

    // update or insert
    pub async fn upsert_trade(&self, trade: &Trade) -> Result<()> {
        let mut trades_handle = self.dbClient.insert("trades")?;
        trades_handle.write(trade).await;
        trades_handle.end().await?;
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

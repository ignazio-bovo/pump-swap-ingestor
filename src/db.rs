use crate::trades::Trade;
use anyhow::Result;
use clickhouse::Client;
use clickhouse::sql::Bind;
use std::fs;
use tracing::info;

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

    // update or insert
    pub async fn upsert_trade(&self, trade: Trade) -> Result<()> {
        let mut trades_handle = self.dbClient.insert("trades")?;
        trades_handle.write(&trade).await;
        trades_handle.end().await?;
        Ok(())
    }

    pub async fn run_migrations(&self) {
        let migration_sql =
            fs::read_to_string("migrations/001_trades.sql").expect("Migration file not found");

        self.dbClient
            .query(&migration_sql)
            .execute()
            .await
            .expect("Migration setup is necessary for correct setup");

        info!("Migration completed successfully");
    }

    pub fn get_trade(&self) {}
}

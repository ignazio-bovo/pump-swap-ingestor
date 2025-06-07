use crate::events::{BuyEvent, SellEvent};
use anchor_lang::AnchorDeserialize;
use anchor_lang::Discriminator;
use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use clickhouse::Row;
use reqwest::ClientBuilder;
use solana_sdk::clock::UnixTimestamp;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct PumpProcessor {
    sol_price_usd: RwLock<f64>,
    client: reqwest::Client,
    solana_price_url: &'static str,
}

#[derive(Row, Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub amount_lamport: u64,
    pub amount_usd: f64,
    pub is_sell: bool,
    pub user: String,
    pub timestamp: UnixTimestamp,
    pub tx_hash: String,
    pub log_index: usize,
    pub pool: String,
}

impl PumpProcessor {
    pub async fn new() -> Self {
        let self_ = Self {
            client: reqwest::Client::new(),
            solana_price_url: "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd",
            sol_price_usd: RwLock::new(0.0)
        };
        self_.fetch_sol_price().await.expect("Error in fetching initial sol/usd price");
        self_
    }
    pub async fn deserialize_pump(&mut self, data: &mut &[u8], tx_hash: &String, log_index: usize) -> Result<Option<Trade>> {
        if data.len() < 8 {
            tracing::error!("discriminator too short");
        }
        let discriminator = &data[..8];
        let trade = match discriminator {
            BuyEvent::DISCRIMINATOR => {
                let buy_data = BuyEvent::deserialize(data)?;
                let buy = self.process_buy(&buy_data, tx_hash, log_index).await?;
                Some(buy)
            }
            SellEvent::DISCRIMINATOR => {
                let sell_data = SellEvent::deserialize(data)?;
                let sell = self.process_sell(&sell_data, tx_hash, log_index).await?;
                Some(sell)
            }
            _ => None
        };

        Ok(trade)
    }

    pub async fn process_buy(&mut self, data: &BuyEvent, tx_hash: &String, index: usize) -> Result<Trade> {
        let amount_lamport = data.quote_amount_in;
        let amount_usd = self.lamport_to_usd(amount_lamport).await;
        Ok(Trade {
            amount_lamport,
            is_sell: false,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user.to_string(),
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
            amount_usd,
        })
    }

    pub async fn process_sell(&mut self, data: &SellEvent, tx_hash: &String, index: usize) -> Result<Trade> {
        let amount_lamport = data.quote_amount_out;
        let amount_usd = self.lamport_to_usd(amount_lamport).await;
        Ok(Trade {
            amount_lamport,
            is_sell: true,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user.to_string(),
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
            amount_usd,
        })
    }

    pub async fn fetch_sol_price(&self) -> Result<()> {
        let response = self.client.get(String::from(self.solana_price_url)).header("User-Agent", "bonk-sol-price-fetcher").send().await?;
        let json_response: serde_json::Value = response.json().await?;
        if let Some(price) = json_response["solana"]["usd"].as_f64() {
            let mut price_guard = self.sol_price_usd.write().await;
            *price_guard = price;
            return Ok(())
        }

        Err(anyhow!("Error converting price to floating point"))
    }

    // Helper function to get current SOL price
    pub async fn get_sol_usd_price(&self) -> f64 {
        *self.sol_price_usd.read().await
    }

    async fn lamport_to_usd(&self, amount_lamport: u64) -> f64 {
        let amount_lamport = amount_lamport as f64 / 1e9;
        let price_usd_sol = self.get_sol_usd_price().await;
        amount_lamport * price_usd_sol
    }
}

#[cfg(test)]
mod tests {
    use crate::trades::PumpProcessor;

    #[tokio::test]
    async fn fetch_price_for_solana_ok() {
        let proc = PumpProcessor::new().await;
        let res = proc.fetch_sol_price().await;
        assert!(res.is_ok());
    }
}
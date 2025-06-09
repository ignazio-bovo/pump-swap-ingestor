use crate::idl::{BuyEvent, SellEvent, BUY_EVENT_DISCRIMINATOR, SELL_EVENT_DISCRIMINATOR};
use crate::pool::{PoolCache, PoolInfo};
use anchor_lang::AnchorDeserialize;
use anyhow::{anyhow, Result};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use solana_sdk::clock::UnixTimestamp;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::error;

#[derive(Clone)]
pub struct PumpProcessor {
    sol_price_usd: Arc<RwLock<f64>>, // Arc needed for cloning pourposes
    client: reqwest::Client,
    solana_price_url: &'static str,
    sol_mint: Pubkey,
    pool_cache: Arc<PoolCache>,
}

#[derive(Row, Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub amount_lamport: u64,
    pub amount_usd: f64,
    pub is_sell: bool,
    pub user: String,
    pub timestamp_unix: UnixTimestamp,
    pub tx_hash: String,
    pub log_index: usize,
    pub pool: String,
    pub fees: u64,
    pub fees_usd: f64,
    pub quote_mint: String,
    pub base_mint: String,
    pub quote_amount: u64,
    pub base_amount: u64,
}

impl PumpProcessor {
    pub async fn new() -> Self {
        let self_ = Self {
            client: reqwest::Client::new(),
            solana_price_url: "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd",
            sol_price_usd: Arc::new(RwLock::new(0.0)),
            sol_mint: Pubkey::from_str_const("So11111111111111111111111111111111111111112"),
            pool_cache: Arc::new(PoolCache::new()),
        };
        self_
            .fetch_sol_price()
            .await
            .expect("Error in fetching initial sol/usd price");
        self_
    }

    pub async fn deserialize_pump(
        &mut self,
        data: &mut &[u8],
        tx_hash: &String,
        log_index: usize,
    ) -> Result<Option<Trade>> {
        if data.len() < 8 {
            error!("discriminator too short");
        }
        let discriminator: [u8; 8] = data[..8].try_into()?;

        let data_to_deserialize = &mut &data[8..];
        let trade = match discriminator {
            BUY_EVENT_DISCRIMINATOR  => {
                let buy_data = BuyEvent::deserialize(data_to_deserialize)?;
                let buy = self.process_buy(&buy_data, tx_hash, log_index).await?;
                Some(buy)
            }
            SELL_EVENT_DISCRIMINATOR => {
                let sell_data = SellEvent::deserialize(data_to_deserialize)?;
                let sell = self.process_sell(&sell_data, tx_hash, log_index).await?;
                Some(sell)
            }
            _ => None,
        };

        Ok(trade)
    }

    pub async fn process_buy(
        &mut self,
        data: &BuyEvent,
        tx_hash: &String,
        index: usize,
    ) -> Result<Trade> {
        let pool_info = self.pool_cache.get_pool_info(&data.pool).await?;
        let amount_lamport = self.lamport_for_buy(data, &pool_info).await;
        let amount_usd = self.lamport_to_usd(amount_lamport).await;
        let user = data.user.to_string();
        let fees = data.lp_fee + data.coin_creator_fee + data.protocol_fee;
        let fees_usd = self.lamport_to_usd(fees).await;
        Ok(Trade {
            amount_lamport,
            is_sell: false,
            timestamp_unix: UnixTimestamp::from(data.timestamp),
            user,
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
            amount_usd,
            fees,
            fees_usd,
            quote_mint: pool_info.quote_mint.to_string(),
            base_mint: pool_info.base_mint.to_string(),
            quote_amount: data.quote_amount_in,
            base_amount: data.base_amount_out,
        })
    }

    pub async fn process_sell(
        &mut self,
        data: &SellEvent,
        tx_hash: &String,
        index: usize,
    ) -> Result<Trade> {
        let pool_info = self.pool_cache.get_pool_info(&data.pool).await?;
        let amount_lamport = self.lamport_for_sell(data, &pool_info).await;
        let amount_usd = self.lamport_to_usd(amount_lamport).await;
        let user = data.user.to_string();
        let fees = data.lp_fee + data.coin_creator_fee + data.protocol_fee;
        let fees_usd = self.lamport_to_usd(fees).await;
        Ok(Trade {
            amount_lamport,
            is_sell: true,
            timestamp_unix: UnixTimestamp::from(data.timestamp),
            user,
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
            amount_usd,
            fees,
            fees_usd,
            quote_mint: pool_info.quote_mint.to_string(),
            base_mint: pool_info.base_mint.to_string(),
            quote_amount: data.quote_amount_out,
            base_amount: data.base_amount_in,
        })
    }

    pub async fn fetch_sol_price(&self) -> Result<()> {
        let response = self
            .client
            .get(String::from(self.solana_price_url))
            .header("User-Agent", "bonk-sol-price-fetcher")
            .send()
            .await?;
        let json_response: serde_json::Value = response.json().await?;
        if let Some(price) = json_response["solana"]["usd"].as_f64() {
            let mut price_guard = self.sol_price_usd.write().await;
            *price_guard = price;
            return Ok(());
        }

        Err(anyhow!("Error converting price to floating point"))
    }

    // Helper function to get current SOL price
    pub async fn get_sol_usd_price(&self) -> f64 {
        *self.sol_price_usd.read().await
    }

    async fn lamport_for_buy(&self, event: &BuyEvent, info: &PoolInfo) -> u64 {
        if info.quote_mint == self.sol_mint {
            event.quote_amount_in
        } else {
            event.base_amount_out
        }
    }

    async fn lamport_for_sell(&self, event: &SellEvent, info: &PoolInfo) -> u64 {
        if info.quote_mint == self.sol_mint {
            event.quote_amount_out
        } else {
            event.base_amount_in
        }
    }

    async fn lamport_to_usd(&self, amount_lamport: u64) -> f64 {
        let amount_sol = amount_lamport as f64 / 1e9;
        let price_usd_sol = self.get_sol_usd_price().await;
        let usd_amount = amount_sol * price_usd_sol;
        (usd_amount * 100.0).round() / 100.0
    }

    pub async fn price_update_service(&self) {
        let processor = self.clone();

        tokio::spawn(async move {
            // fetch every 20 seconds
            let mut interval = tokio::time::interval(Duration::from_secs(20));

            loop {
                interval.tick().await;

                if let Err(e) = processor.fetch_sol_price().await {
                    error!("Failed to update SOL price: {}", e);
                }
            }
        });
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

use crate::events::{BuyEvent, SellEvent};
use anchor_lang::AnchorDeserialize;
use anchor_lang::Discriminator;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use clickhouse::Row;
use solana_sdk::clock::UnixTimestamp;
use tracing::{error, info, warn};

pub struct PumpDeserialize {
}

#[derive(Row, Clone, Debug, Serialize, Deserialize)]
pub struct Trade {
    pub amount_sol: u64,
    pub is_sell: bool,
    pub user: String,
    pub timestamp: UnixTimestamp,
    pub tx_hash: String,
    pub log_index: usize,
    pub pool: String,
}

impl PumpDeserialize {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn deserialize_pump(&mut self, data: &mut &[u8], tx_hash: &String, log_index: usize) -> Result<Option<Trade>> {
        if data.len() < 8 {
            tracing::error!("discriminator too short");
        }
        let discriminator = &data[..8];
        let trade = match discriminator {
            BuyEvent::DISCRIMINATOR => {
                let buy_data = BuyEvent::deserialize(data)?;
                let buy = self.process_buy(&buy_data, tx_hash, log_index)?;
                Some(buy)
            }
            SellEvent::DISCRIMINATOR => {
                let sell_data = SellEvent::deserialize(data)?;
                let sell = self.process_sell(&sell_data, tx_hash, log_index)?;
                Some(sell)
            }
            _ =>  None
        };

        Ok(trade)
    }

    pub fn process_buy(&mut self, data: &BuyEvent, tx_hash: &String, index: usize) -> Result<Trade> {
        Ok(Trade {
            amount_sol: data.quote_amount_in,
            is_sell: false,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user.to_string(),
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
        })
    }

    pub fn process_sell(&mut self, data: &SellEvent, tx_hash: &String, index: usize) -> Result<Trade> {
        Ok(Trade {
            amount_sol: data.quote_amount_out,
            is_sell: true,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user.to_string(),
            tx_hash: tx_hash.to_owned(),
            log_index: index,
            pool: data.pool.to_string(),
        })
    }
}

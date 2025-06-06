use crate::events::{BuyEvent, SellEvent};
use crate::pool::PoolCache;
use anchor_lang::AnchorDeserialize;
use anchor_lang::Discriminator;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use solana_sdk::clock::UnixTimestamp;
use tracing::{info, warn};

pub struct PumpDeserialize {
    cache: PoolCache,
}

#[derive(Clone, Debug)]
struct Trade {
    input_token: Pubkey,
    output_token: Pubkey,
    amount_in: u64,
    amount_out: u64,
    is_sell: bool,
    user: Pubkey,
    timestamp: UnixTimestamp,
}

impl PumpDeserialize {
    pub fn new() -> Self {
        Self {
            cache: PoolCache::new(),
        }
    }
    pub fn deserialize_pump(&mut self, data: &mut &[u8]) -> Result<()> {
        if data.len() < 8 {
            tracing::error!("discriminator too short");
        }
        let discriminator = &data[..8];
        match discriminator {
            BuyEvent::DISCRIMINATOR => {
                let buy_data = BuyEvent::deserialize(data)?;
                warn!("Buy event found {:?}", buy_data);
                let buy = self.process_buy(&buy_data)?;
            }
            SellEvent::DISCRIMINATOR => {
                let sell_data = SellEvent::deserialize(data)?;
                warn!("Sell event found {:?}", sell_data);
                let sell = self.process_sell(&sell_data)?;
            }
            _ => {}
        }

        Ok(())
    }

    pub fn process_buy(&mut self, data: &BuyEvent) -> Result<Trade> {
        let pool = self.cache.get_pool_info(&data.pool)?;
        Ok(Trade {
            input_token: pool.quote_token,
            output_token: pool.base_token,
            amount_in: data.quote_amount_in,
            amount_out: data.base_amount_out,
            is_sell: false,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user,
        })
    }

    pub fn process_sell(&mut self, data: &SellEvent) -> Result<Trade> {
        let pool = self.cache.get_pool_info(&data.pool)?;
        Ok(Trade {
            input_token: pool.base_token,
            output_token: pool.quote_token,
            amount_in: data.base_amount_in,
            amount_out: data.quote_amount_out,
            is_sell: true,
            timestamp: UnixTimestamp::from(data.timestamp),
            user: data.user,
        })
    }
}

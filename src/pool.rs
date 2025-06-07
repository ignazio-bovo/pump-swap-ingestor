use anyhow::{anyhow, Result};
use crate::idl::Pool;
use anchor_lang::AnchorDeserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::collections::HashMap;
use std::sync::Arc;
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Clone, PartialEq, Debug)]
pub struct PoolInfo {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
}

pub struct PoolCache {
    data: Arc<RwLock<HashMap<Pubkey, PoolInfo>>>, // Wrapped in RwLock
    client: RpcClient,
}

impl PoolCache {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            client: RpcClient::new(String::from("https://api.mainnet-beta.solana.com")),
        }
    }

    pub async fn get_pool_info(&self, pool_id: &Pubkey) -> Result<PoolInfo> {
        { // interior mutability
            let cache = self.data.read().await;
            if let Some(pool) = cache.get(pool_id) {
                return Ok(pool.clone());
            }
        }

        self.fetch_pool_info(pool_id).await
    }

    async fn fetch_pool_info(&self, id: &Pubkey) -> Result<PoolInfo> {
        let data = self
            .client
            .get_account_data(id)
            .await
            .map_err(|_| anyhow!("Pool account not found"))?;

        let mut data_slice = &mut data.as_slice();
        if data_slice.len() < 8 {
            return Err(anyhow!("Error with the pool fetching"));
        }

        let data_slice_to_deserialize = &mut &data_slice[8..];

        let pool = Pool::deserialize(data_slice_to_deserialize)?;
        let pool_info = PoolInfo {
            base_mint: pool.base_mint,
            quote_mint: pool.quote_mint,
        };

        // Insert into cache with write lock
        {
            let mut cache = self.data.write().await;
            cache.insert(*id, pool_info.clone());
        }

        info!("Pool added to cache {:?}", id);
        Ok(pool_info)
    }
}

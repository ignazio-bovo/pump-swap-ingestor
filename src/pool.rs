use anchor_lang::AnchorDeserialize;
use anchor_lang::prelude::Pubkey;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use solana_client::rpc_client::RpcClient;
use tracing::{info, warn};

#[derive(Clone, PartialEq, Debug)]
pub struct PoolInfo {
    pub base_token: Pubkey,
    pub quote_token: Pubkey,
}

pub struct PoolCache {
    data: HashMap<Pubkey, PoolInfo>,
    client: RpcClient,
}

impl PoolCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            client: RpcClient::new("https://api.mainnet-beta.solana.com"),
        }
    }

    pub fn get_pool_info(&mut self, pool_id: &Pubkey) -> Result<PoolInfo> {
        if let Some(pool) = self.data.get(pool_id) {
            Ok(pool.to_owned())
        } else {
            // cache miss
            warn!("getting pool info at {:?}", pool_id);
            self.fetch_pool_info(pool_id)
        }
    }

    fn fetch_pool_info(&mut self, id: &Pubkey) -> Result<PoolInfo> {
        let data = self.client.get_account_data(id).map_err(|_| anyhow!("Pool account not found"))?;
        let mut data_slice = &mut data.as_slice();
        if data_slice.len() < 8 {
            return Err(anyhow!("Error with the pool fetching"));
        }
        let pool = crate::typedefs::Pool::deserialize(data_slice)?;
        let pool_info = PoolInfo {
            base_token: pool.base_mint,
            quote_token: pool.quote_mint,
        };
        self.data.insert(id.to_owned(), pool_info.clone());
        info!("Pool added to cache {:?}", id);
        Ok(pool_info)
    }
}

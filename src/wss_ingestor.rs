use anyhow::Result;
use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;
use tracing::{error, info, warn};
use crate::trades::PumpDeserialize;

pub struct WssIngestor {
    url: String,
    program_id: String,
    deserializer: PumpDeserialize,
}

impl WssIngestor {
    pub fn new(url: &str, program_id: &str) -> Self {
        Self {
            url: url.to_string(),
            program_id: program_id.to_string(),
            deserializer: PumpDeserialize::new(),
        }
    }

    pub fn ingest_trades(&mut self) -> Result<()> {
        let (_client, receiver) = PubsubClient::logs_subscribe(
            "wss://api.mainnet-beta.solana.com",
            RpcTransactionLogsFilter::Mentions(vec![self.program_id.clone()]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )?;

        info!("🚀 Listening for trades on {}", self.program_id);

        loop {
            match receiver.recv_timeout(Duration::from_secs(30)) {
                Ok(response) => {
                    for log in &response.value.logs {
                        if log.starts_with("Program data: ") {
                            let base64_data = log.strip_prefix("Program data: ").unwrap().trim();

                            if let Ok(raw_data) = base64::decode(base64_data) {
                                let data_slice = &mut raw_data.as_slice();
                                let trade = self.deserializer.deserialize_pump(data_slice);
                                if trade.is_err() {
                                    tracing::error!("Error parsing trade {:?}", trade);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    info!("⏰ No activity in 30s");
                }
            }
        }
    }
}

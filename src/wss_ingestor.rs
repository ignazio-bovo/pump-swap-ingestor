use crate::db::BackendDb;
use crate::trades::{PumpDeserialize, Trade};
use anyhow::Result;
use solana_client::pubsub_client::{LogsSubscription, PubsubClient};
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

pub struct WssIngestor {
    url: String,
    program_id: String,
    deserializer: PumpDeserialize,
    out_channel: UnboundedSender<Trade>,
    log_subscription: LogsSubscription,
}

impl WssIngestor {
    pub fn new(url: &str, program_id: &str, out_channel: UnboundedSender<Trade>) -> Result<Self> {
        let log_subscription = PubsubClient::logs_subscribe(
            "wss://api.mainnet-beta.solana.com",
            RpcTransactionLogsFilter::Mentions(vec![String::from(program_id)]),
            RpcTransactionLogsConfig {
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )?;
        Ok(Self {
            url: url.to_string(),
            program_id: program_id.to_string(),
            deserializer: PumpDeserialize::new(),
            out_channel,
            log_subscription,
        })
    }

    pub async fn ingest_trades(&mut self) {
        info!("🚀 Listening for trades on {}", self.program_id);

        loop {
            match self
                .log_subscription
                .1
                .recv_timeout(Duration::from_secs(30))
            {
                Ok(response) => {
                    let tx_hash = response.value.signature.clone();

                    for (log_index, log) in response.value.logs.iter().enumerate() {
                        if log.starts_with("Program data: ") {
                            let base64_data = log.strip_prefix("Program data: ").unwrap().trim();

                            if let Ok(raw_data) = base64::decode(base64_data) {
                                let data_slice = &mut raw_data.as_slice();
                                if let Ok(maybe_trade) = self
                                    .deserializer
                                    .deserialize_pump(data_slice, &tx_hash, log_index)
                                {
                                    info!("Correctly producedtrade {:?}", tx_hash);
                                    maybe_trade.map(|trade| {
                                        if let Err(e) = self.out_channel.send(trade) {
                                            error!("Failure in sending trade {:?} to db", tx_hash);
                                        }
                                    });
                                } else {
                                    tracing::error!("Error parsing trade");
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

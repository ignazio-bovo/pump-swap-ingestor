use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use crate::trades::{PumpProcessor, Trade};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

pub struct WssIngestor {
    url: String,
    program_id: String,
    deserializer: PumpProcessor,
}

impl WssIngestor {
    pub async fn new(url: &str, program_id: &str) -> Result<Self> {
        Ok(Self {
            url: url.to_string(),
            program_id: program_id.to_string(),
            deserializer: PumpProcessor::new().await,
        })
    }

    pub async fn ingest_trades(&mut self, tx: UnboundedSender<Trade>) -> Result<()> {
        info!("🚀 Connecting to {}", self.url);

        let (ws_stream, _) = connect_async(&self.url).await.expect("Issue with connecting to the Solana endpoint");
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to logs
        let subscription = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "logsSubscribe",
            "params": [
                {"mentions": [self.program_id.clone()]},
                {"commitment": "confirmed"}
            ]
        });

        if let Err(e) = ws_sender.send(Message::Text(subscription.to_string())).await {
            error!("error with the wss connection {:?}", e.to_string());
        }
        info!("🚀 Subscribed to logs for {}", self.program_id);

        self.deserializer.price_update_service().await;
        info!("👷Starting USD price update");

        while let Some(message) = ws_receiver.next().await {
            match message? {
                Message::Text(text) => {
                    if let Err(e) = self.process_message(&text, &tx).await {
                        error!("Error processing message: {}", e);
                    }
                }
                Message::Close(_) => {
                    info!("WebSocket connection closed");
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_message(&mut self, message: &str, tx: &UnboundedSender<Trade>) -> Result<()> {
        let parsed: serde_json::Value = serde_json::from_str(message)?;

        // Parse the Solana WebSocket response format
        if let Some(params) = parsed.get("params") {
            if let Some(result) = params.get("result") {
                if let Some(logs) = result.get("value").and_then(|v| v.get("logs")) {
                    let tx_hash = result
                        .get("value")
                        .and_then(|v| v.get("signature"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    if let Some(logs_array) = logs.as_array() {
                        for (log_index, log) in logs_array.iter().enumerate() {
                            if let Some(log_str) = log.as_str() {
                                if log_str.starts_with("Program data: ") {
                                    let base64_data = log_str.strip_prefix("Program data: ").unwrap().trim();

                                    if let Ok(raw_data) = base64::decode(base64_data) {
                                        let data_slice = &mut raw_data.as_slice();
                                        if let Ok(maybe_trade) = self
                                            .deserializer
                                            .deserialize_pump(data_slice, &tx_hash, log_index).await
                                        {
                                            if let Some(trade) = maybe_trade {
                                                if let Err(e) = tx.send(trade) {
                                                    error!("Channel closed: {}", e);
                                                    return Err(anyhow::anyhow!("Channel closed"));
                                                }
                                                debug!("✅ Trade sent for tx: {}", tx_hash);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
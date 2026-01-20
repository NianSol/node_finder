use super::{parse_hex_u64, RpcRequest, RpcResponse, ValidatedNode, HTTP_TIMEOUT, WS_SEMAPHORE_LIMIT};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct WsValidator {
    semaphore: Arc<Semaphore>,
}

impl WsValidator {
    pub fn new() -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(WS_SEMAPHORE_LIMIT)),
        }
    }

    async fn rpc_call(&self, url: &str, request: &RpcRequest) -> Result<RpcResponse, String> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| "Semaphore error")?;

        let (mut ws_stream, _) = timeout(HTTP_TIMEOUT, connect_async(url))
            .await
            .map_err(|_| "WS connection timeout")?
            .map_err(|e| format!("WS connection error: {}", e))?;

        let request_json =
            serde_json::to_string(request).map_err(|e| format!("JSON serialize error: {}", e))?;

        ws_stream
            .send(Message::Text(request_json))
            .await
            .map_err(|e| format!("WS send error: {}", e))?;

        let response = timeout(HTTP_TIMEOUT, ws_stream.next())
            .await
            .map_err(|_| "WS response timeout")?
            .ok_or("No WS response")?
            .map_err(|e| format!("WS receive error: {}", e))?;

        let response_text = match response {
            Message::Text(t) => t,
            _ => return Err("Unexpected WS message type".to_string()),
        };

        serde_json::from_str(&response_text).map_err(|e| format!("JSON parse error: {}", e))
    }

    pub async fn validate(
        &self,
        url: &str,
        expected_chain_id: u64,
        expected_genesis_hash: &str,
        reference_block: u64,
        sync_tolerance: u64,
    ) -> Result<ValidatedNode, String> {
        let start = Instant::now();

        // Check chain ID
        let chain_id_resp = self.rpc_call(url, &RpcRequest::eth_chain_id()).await?;
        let chain_id_hex = chain_id_resp
            .result
            .and_then(|v| v.as_str().map(String::from))
            .ok_or("No chain ID in response")?;
        let chain_id = parse_hex_u64(&chain_id_hex).ok_or("Invalid chain ID format")?;

        if chain_id != expected_chain_id {
            return Err(format!(
                "Chain ID mismatch: expected {}, got {}",
                expected_chain_id, chain_id
            ));
        }

        // Check genesis block hash
        let genesis_resp = self
            .rpc_call(url, &RpcRequest::eth_get_block_by_number("0x0", false))
            .await?;
        let genesis_hash = genesis_resp
            .result
            .and_then(|v| v.get("hash").and_then(|h| h.as_str()).map(String::from))
            .ok_or("No genesis hash in response")?;

        if genesis_hash.to_lowercase() != expected_genesis_hash.to_lowercase() {
            return Err("Genesis hash mismatch - possible honeypot".to_string());
        }

        // Check sync status
        let block_resp = self.rpc_call(url, &RpcRequest::eth_block_number()).await?;
        let block_hex = block_resp
            .result
            .and_then(|v| v.as_str().map(String::from))
            .ok_or("No block number in response")?;
        let block_number = parse_hex_u64(&block_hex).ok_or("Invalid block number format")?;

        let block_diff = if reference_block > block_number {
            reference_block - block_number
        } else {
            block_number - reference_block
        };

        if block_diff > sync_tolerance {
            return Err(format!(
                "Node not synced: {} blocks behind (tolerance: {})",
                block_diff, sync_tolerance
            ));
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(ValidatedNode {
            url: url.to_string(),
            latency_ms,
            block_number,
            is_archive: false,
        })
    }
}

impl Default for WsValidator {
    fn default() -> Self {
        Self::new()
    }
}

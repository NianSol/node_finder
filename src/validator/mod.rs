pub mod http;
pub mod ws;
pub mod archive;

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
pub const ARCHIVE_TIMEOUT: Duration = Duration::from_secs(10);
pub const WS_SEMAPHORE_LIMIT: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
    pub id: u64,
}

impl RpcRequest {
    pub fn new(method: &str, params: Vec<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: 1,
        }
    }

    pub fn eth_chain_id() -> Self {
        Self::new("eth_chainId", vec![])
    }

    pub fn eth_block_number() -> Self {
        Self::new("eth_blockNumber", vec![])
    }

    pub fn eth_get_block_by_number(block: &str, full_tx: bool) -> Self {
        Self::new(
            "eth_getBlockByNumber",
            vec![serde_json::json!(block), serde_json::json!(full_tx)],
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcResponse {
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedNode {
    pub url: String,
    pub latency_ms: u64,
    pub block_number: u64,
    pub is_archive: bool,
}

pub fn parse_hex_u64(s: &str) -> Option<u64> {
    let s = s.trim_start_matches("0x");
    u64::from_str_radix(s, 16).ok()
}

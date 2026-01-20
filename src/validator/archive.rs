use super::{RpcRequest, RpcResponse, ValidatedNode, ARCHIVE_TIMEOUT};
use reqwest::Client;
use std::time::Duration;
use tokio::time::timeout;

const ARCHIVE_BLOCKS: [&str; 3] = ["0x1", "0x64", "0xf4240"]; // 1, 100, 1000000

pub struct ArchiveValidator {
    client: Client,
}

impl ArchiveValidator {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15)) // Higher timeout for archive queries
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    async fn rpc_call(&self, url: &str, request: &RpcRequest) -> Result<RpcResponse, String> {
        let response = self
            .client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP status: {}", response.status()));
        }

        response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))
    }

    /// Check if a node is an archive node by querying early blocks.
    /// All three blocks (1, 100, 1000000) must return valid data.
    pub async fn check_archive(&self, url: &str) -> Result<bool, String> {
        let check = async {
            for block in ARCHIVE_BLOCKS {
                let request = RpcRequest::eth_get_block_by_number(block, false);
                let response = self.rpc_call(url, &request).await?;

                // Check if we got a valid block response (not null, not error)
                match response.result {
                    Some(value) if !value.is_null() => {
                        // Verify the response has expected block fields
                        if value.get("number").is_none() || value.get("hash").is_none() {
                            return Err(format!("Block {} returned incomplete data", block));
                        }
                    }
                    _ => {
                        return Err(format!("Block {} not available", block));
                    }
                }
            }
            Ok(true)
        };

        timeout(ARCHIVE_TIMEOUT, check)
            .await
            .map_err(|_| "Archive check timeout".to_string())?
    }

    /// Validate a node as an archive node.
    /// Takes a pre-validated node and checks if it has archive capabilities.
    pub async fn validate_archive(&self, mut node: ValidatedNode) -> Result<ValidatedNode, String> {
        let is_archive = self.check_archive(&node.url).await?;
        node.is_archive = is_archive;
        Ok(node)
    }
}

impl Default for ArchiveValidator {
    fn default() -> Self {
        Self::new()
    }
}

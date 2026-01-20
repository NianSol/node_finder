pub mod storage;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub default_count: u32,
    pub protocol: Protocol,
    pub sync_tolerance: u64,
    pub reference_rpcs: HashMap<u64, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Ws,
}

impl Default for UserConfig {
    fn default() -> Self {
        let mut reference_rpcs = HashMap::new();
        reference_rpcs.insert(1, "https://eth.llamarpc.com".to_string());
        reference_rpcs.insert(56, "https://bsc.meowrpc.com".to_string());
        reference_rpcs.insert(8453, "https://base-rpc.publicnode.com".to_string());

        Self {
            default_count: 10,
            protocol: Protocol::Http,
            sync_tolerance: 50,
            reference_rpcs,
        }
    }
}

impl UserConfig {
    pub fn get_reference_rpc(&self, chain_id: u64) -> Option<&String> {
        self.reference_rpcs.get(&chain_id)
    }
}

pub mod genesis;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: u64,
    pub name: String,
    pub symbol: String,
    pub default_rpc: String,
    pub genesis_hash: String,
}

impl Chain {
    pub fn hex_id(&self) -> String {
        format!("0x{:x}", self.id)
    }
}

pub fn get_default_chains() -> Vec<Chain> {
    vec![
        Chain {
            id: 1,
            name: "Ethereum".to_string(),
            symbol: "Ξ".to_string(),
            default_rpc: "https://eth.llamarpc.com".to_string(),
            genesis_hash: genesis::ETH_GENESIS.to_string(),
        },
        Chain {
            id: 56,
            name: "BSC".to_string(),
            symbol: "⛓️".to_string(),
            default_rpc: "https://bsc.meowrpc.com".to_string(),
            genesis_hash: genesis::BSC_GENESIS.to_string(),
        },
        Chain {
            id: 8453,
            name: "Base".to_string(),
            symbol: "🔵".to_string(),
            default_rpc: "https://base-rpc.publicnode.com".to_string(),
            genesis_hash: genesis::BASE_GENESIS.to_string(),
        },
    ]
}

pub fn get_chain_by_id(id: u64) -> Option<Chain> {
    get_default_chains().into_iter().find(|c| c.id == id)
}

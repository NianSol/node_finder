use crate::chains::Chain;
use crate::config::storage::ConfigManager;
use crate::shodan::ShodanClient;
use crate::validator::{archive::ArchiveValidator, http::HttpValidator, ws::WsValidator};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeType {
    Full,
    Archive,
    Bulk,
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub node_type: Option<NodeType>,
    pub chain: Option<Chain>,
    pub custom_chain_id: Option<u64>,
    pub awaiting_chain_id: bool,
    pub awaiting_rpc_url: bool,
}

impl Default for UserSession {
    fn default() -> Self {
        Self {
            node_type: None,
            chain: None,
            custom_chain_id: None,
            awaiting_chain_id: false,
            awaiting_rpc_url: false,
        }
    }
}

#[derive(Clone)]
pub struct BotState {
    pub shodan: ShodanClient,
    pub config_manager: ConfigManager,
    pub http_validator: Arc<HttpValidator>,
    pub ws_validator: Arc<WsValidator>,
    pub archive_validator: Arc<ArchiveValidator>,
    pub sessions: Arc<RwLock<HashMap<i64, UserSession>>>,
}

impl BotState {
    pub fn new(shodan_token: String) -> Self {
        Self {
            shodan: ShodanClient::new(shodan_token),
            config_manager: ConfigManager::new(),
            http_validator: Arc::new(HttpValidator::new()),
            ws_validator: Arc::new(WsValidator::new()),
            archive_validator: Arc::new(ArchiveValidator::new()),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_session(&self, user_id: i64) -> UserSession {
        let sessions = self.sessions.read().await;
        sessions.get(&user_id).cloned().unwrap_or_default()
    }

    pub async fn set_session(&self, user_id: i64, session: UserSession) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(user_id, session);
    }

    pub async fn clear_session(&self, user_id: i64) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&user_id);
    }

    pub async fn update_session<F>(&self, user_id: i64, f: F)
    where
        F: FnOnce(&mut UserSession),
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions.entry(user_id).or_insert_with(UserSession::default);
        f(session);
    }
}

pub struct Location {
    pub code: &'static str,
    pub name: &'static str,
    pub flag: &'static str,
}

pub const LOCATIONS: &[Location] = &[
    Location { code: "US", name: "United States", flag: "🇺🇸" },
    Location { code: "DE", name: "Germany", flag: "🇩🇪" },
    Location { code: "FI", name: "Finland", flag: "🇫🇮" },
    Location { code: "CA", name: "Canada", flag: "🇨🇦" },
    Location { code: "NL", name: "Netherlands", flag: "🇳🇱" },
    Location { code: "FR", name: "France", flag: "🇫🇷" },
    Location { code: "SG", name: "Singapore", flag: "🇸🇬" },
];

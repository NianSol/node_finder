use super::UserConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigStore {
    users: HashMap<i64, UserConfig>,
}

#[derive(Debug, Clone)]
pub struct ConfigManager {
    store: Arc<RwLock<ConfigStore>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let store = if Path::new(CONFIG_FILE).exists() {
            match fs::read_to_string(CONFIG_FILE) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => ConfigStore::default(),
            }
        } else {
            ConfigStore::default()
        };

        Self {
            store: Arc::new(RwLock::new(store)),
        }
    }

    pub async fn get_user_config(&self, user_id: i64) -> UserConfig {
        let store = self.store.read().await;
        store.users.get(&user_id).cloned().unwrap_or_default()
    }

    pub async fn set_user_config(&self, user_id: i64, config: UserConfig) {
        let mut store = self.store.write().await;
        store.users.insert(user_id, config);
        self.save_store(&store);
    }

    pub async fn update_user_config<F>(&self, user_id: i64, f: F)
    where
        F: FnOnce(&mut UserConfig),
    {
        let mut store = self.store.write().await;
        let config = store.users.entry(user_id).or_insert_with(UserConfig::default);
        f(config);
        self.save_store(&store);
    }

    fn save_store(&self, store: &ConfigStore) {
        if let Ok(content) = serde_json::to_string_pretty(store) {
            let _ = fs::write(CONFIG_FILE, content);
        }
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

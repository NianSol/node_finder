use reqwest::Client;
use serde::Deserialize;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

const SHODAN_API_BASE: &str = "https://api.shodan.io";
const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct ShodanResult {
    pub ip: String,
    pub port: u16,
    pub country_code: Option<String>,
}

impl ShodanResult {
    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.ip, self.port)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", self.ip, self.port)
    }

    pub fn is_http_port(&self) -> bool {
        self.port == 8545
    }

    pub fn is_ws_port(&self) -> bool {
        self.port == 8546
    }
}

#[derive(Debug, Deserialize)]
struct ShodanSearchResponse {
    matches: Vec<ShodanMatch>,
}

#[derive(Debug, Deserialize)]
struct ShodanMatch {
    ip_str: String,
    port: u16,
    location: ShodanLocation,
}

#[derive(Debug, Deserialize)]
struct ShodanLocation {
    country_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShodanClient {
    client: Client,
    api_key: String,
    last_request: Arc<Mutex<Option<Instant>>>,
}

impl ShodanClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            last_request: Arc::new(Mutex::new(None)),
        }
    }

    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < RATE_LIMIT_INTERVAL {
                tokio::time::sleep(RATE_LIMIT_INTERVAL - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }

    pub async fn search_nodes(
        &self,
        chain_id: u64,
        country_code: Option<&str>,
    ) -> Result<Vec<ShodanResult>, String> {
        self.rate_limit().await;

        let hex_id = format!("0x{:x}", chain_id);
        let decimal_id = chain_id.to_string();

        // Build query: search for both hex and decimal chain IDs, both ports
        let mut query = format!(
            "port:8545,8546 (\"Chain ID: {}\" OR \"Chain ID: {}\")",
            hex_id, decimal_id
        );

        if let Some(cc) = country_code {
            query.push_str(&format!(" country:{}", cc));
        }

        let encoded_query: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let url = format!(
            "{}/shodan/host/search?key={}&query={}",
            SHODAN_API_BASE,
            self.api_key,
            encoded_query
        );

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Shodan request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Shodan API error: {}", response.status()));
        }

        let data: ShodanSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Shodan response: {}", e))?;

        let results: Vec<ShodanResult> = data
            .matches
            .into_iter()
            .map(|m| ShodanResult {
                ip: m.ip_str,
                port: m.port,
                country_code: m.location.country_code,
            })
            .collect();

        Ok(results)
    }
}


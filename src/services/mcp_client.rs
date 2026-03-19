use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone)]
pub struct MCPServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
    pub description: String,
    pub enabled: bool,
}

/// Placeholder MCP client - manages server registry but actual connections return 503.
#[derive(Clone)]
pub struct MCPClient {
    servers: Arc<RwLock<HashMap<String, MCPServerConfig>>>,
}

impl MCPClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn is_available(&self) -> bool {
        // MCP SDK is not available in Rust - placeholder
        false
    }

    pub async fn register_server(&self, config: MCPServerConfig) {
        let mut servers = self.servers.write().await;
        info!("Registered MCP server: {}", config.name);
        servers.insert(config.name.clone(), config);
    }

    pub async fn list_servers(&self) -> Vec<MCPServerConfig> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    pub async fn get_stats(&self) -> serde_json::Value {
        let servers = self.servers.read().await;
        serde_json::json!({
            "mcp_sdk_available": false,
            "registered_servers": servers.len(),
            "connected_servers": 0,
            "message": "MCP SDK not available in Rust build. Server registration works but connections are not supported."
        })
    }
}

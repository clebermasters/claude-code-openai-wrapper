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

    pub async fn remove_server(&self, name: &str) -> bool {
        self.servers.write().await.remove(name).is_some()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_not_available() {
        let client = MCPClient::new();
        assert!(!client.is_available());
    }

    #[tokio::test]
    async fn test_register_and_list() {
        let client = MCPClient::new();
        client.register_server(MCPServerConfig {
            name: "test".into(),
            command: "npx".into(),
            args: vec![],
            env: None,
            description: "test server".into(),
            enabled: true,
        }).await;
        let servers = client.list_servers().await;
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test");
    }

    #[tokio::test]
    async fn test_list_empty() {
        let client = MCPClient::new();
        assert!(client.list_servers().await.is_empty());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let client = MCPClient::new();
        let stats = client.get_stats().await;
        assert_eq!(stats["mcp_sdk_available"], false);
        assert_eq!(stats["registered_servers"], 0);
    }

    #[tokio::test]
    async fn test_remove_server() {
        let client = MCPClient::new();
        client.register_server(MCPServerConfig {
            name: "s1".into(), command: "cmd".into(), args: vec![], env: None, description: String::new(), enabled: true,
        }).await;
        assert!(client.remove_server("s1").await);
        assert!(!client.remove_server("s1").await);
        assert!(client.list_servers().await.is_empty());
    }
}

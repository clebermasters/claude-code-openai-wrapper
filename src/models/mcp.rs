use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct MCPServerConfigRequest {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl MCPServerConfigRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Server name cannot be empty".to_string());
        }
        if self.name.len() > 100 {
            return Err("Server name too long (max 100 characters)".to_string());
        }
        if !self.name.chars().all(|c| c.is_alphanumeric() || "-_.".contains(c)) {
            return Err("Server name must contain only alphanumeric characters, hyphens, underscores, and dots".to_string());
        }
        if self.command.trim().is_empty() {
            return Err("Command cannot be empty".to_string());
        }
        if self.command.len() > 500 {
            return Err("Command path too long (max 500 characters)".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MCPServerInfoResponse {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub description: String,
    pub enabled: bool,
    pub connected: bool,
    pub tools_count: usize,
    pub resources_count: usize,
    pub prompts_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MCPServersListResponse {
    pub servers: Vec<MCPServerInfoResponse>,
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MCPConnectionRequest {
    pub server_name: String,
}

impl MCPConnectionRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.server_name.trim().is_empty() {
            return Err("Server name cannot be empty".to_string());
        }
        if self.server_name.len() > 100 {
            return Err("Server name too long (max 100 characters)".to_string());
        }
        Ok(())
    }
}

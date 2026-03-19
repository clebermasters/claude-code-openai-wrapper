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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_valid() {
        let cfg: MCPServerConfigRequest = serde_json::from_str(r#"{"name":"my-server","command":"npx","args":["mcp"],"description":"test"}"#).unwrap();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_server_config_empty_name() {
        let cfg = MCPServerConfigRequest { name: "".into(), command: "npx".into(), args: vec![], env: None, description: String::new(), enabled: true };
        assert!(cfg.validate().unwrap_err().contains("empty"));
    }

    #[test]
    fn test_server_config_name_too_long() {
        let cfg = MCPServerConfigRequest { name: "a".repeat(101), command: "npx".into(), args: vec![], env: None, description: String::new(), enabled: true };
        assert!(cfg.validate().unwrap_err().contains("too long"));
    }

    #[test]
    fn test_server_config_name_invalid_chars() {
        let cfg = MCPServerConfigRequest { name: "bad name!".into(), command: "npx".into(), args: vec![], env: None, description: String::new(), enabled: true };
        assert!(cfg.validate().unwrap_err().contains("alphanumeric"));
    }

    #[test]
    fn test_server_config_empty_command() {
        let cfg = MCPServerConfigRequest { name: "ok".into(), command: "".into(), args: vec![], env: None, description: String::new(), enabled: true };
        assert!(cfg.validate().unwrap_err().contains("Command"));
    }

    #[test]
    fn test_connection_request_valid() {
        let req = MCPConnectionRequest { server_name: "my-server".into() };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_connection_request_empty() {
        let req = MCPConnectionRequest { server_name: "   ".into() };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_connection_request_too_long() {
        let req = MCPConnectionRequest { server_name: "x".repeat(101) };
        assert!(req.validate().is_err());
    }
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

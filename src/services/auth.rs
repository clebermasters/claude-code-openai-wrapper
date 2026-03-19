use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct ClaudeCodeAuthManager {
    pub auth_method: String,
    pub auth_status: AuthStatus,
    api_key: Option<String>,
    pub runtime_api_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub method: String,
    pub valid: bool,
    pub errors: Vec<String>,
    pub config: HashMap<String, serde_json::Value>,
}

impl ClaudeCodeAuthManager {
    pub fn new(config: &Config) -> Self {
        let api_key = config.api_key.clone();
        let auth_method = Self::detect_auth_method(config);
        let auth_status = Self::validate_auth_method(&auth_method);

        info!("Claude Code authentication method: {auth_method}");

        Self {
            auth_method,
            auth_status,
            api_key,
            runtime_api_key: None,
        }
    }

    pub fn get_api_key(&self) -> Option<&str> {
        self.runtime_api_key
            .as_deref()
            .or(self.api_key.as_deref())
    }

    pub fn set_runtime_api_key(&mut self, key: String) {
        self.runtime_api_key = Some(key);
    }

    fn detect_auth_method(config: &Config) -> String {
        if let Some(method) = &config.claude_auth_method {
            let method_lower = method.to_lowercase();
            let mapped = match method_lower.as_str() {
                "cli" | "claude_cli" => "claude_cli",
                "api_key" | "anthropic" => "anthropic",
                "bedrock" => "bedrock",
                "vertex" => "vertex",
                _ => {
                    warn!("Unknown CLAUDE_AUTH_METHOD '{method}', falling back to auto-detect");
                    ""
                }
            };
            if !mapped.is_empty() {
                info!("Using explicit auth method: {mapped}");
                return mapped.to_string();
            }
        }

        // Auto-detect
        if std::env::var("CLAUDE_CODE_USE_BEDROCK").as_deref() == Ok("1") {
            return "bedrock".to_string();
        }
        if std::env::var("CLAUDE_CODE_USE_VERTEX").as_deref() == Ok("1") {
            return "vertex".to_string();
        }
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            return "anthropic".to_string();
        }
        "claude_cli".to_string()
    }

    fn validate_auth_method(method: &str) -> AuthStatus {
        match method {
            "anthropic" => Self::validate_anthropic(),
            "bedrock" => Self::validate_bedrock(),
            "vertex" => Self::validate_vertex(),
            "claude_cli" => AuthStatus {
                method: "claude_cli".to_string(),
                valid: true,
                errors: vec![],
                config: HashMap::from([
                    ("method".to_string(), serde_json::json!("Claude Code CLI authentication")),
                ]),
            },
            _ => AuthStatus {
                method: method.to_string(),
                valid: false,
                errors: vec!["No Claude Code authentication method configured".to_string()],
                config: HashMap::new(),
            },
        }
    }

    fn validate_anthropic() -> AuthStatus {
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(key) if key.len() >= 10 => AuthStatus {
                method: "anthropic".to_string(),
                valid: true,
                errors: vec![],
                config: HashMap::from([
                    ("api_key_present".to_string(), serde_json::json!(true)),
                    ("api_key_length".to_string(), serde_json::json!(key.len())),
                ]),
            },
            Ok(_) => AuthStatus {
                method: "anthropic".to_string(),
                valid: false,
                errors: vec!["ANTHROPIC_API_KEY appears to be invalid (too short)".to_string()],
                config: HashMap::new(),
            },
            Err(_) => AuthStatus {
                method: "anthropic".to_string(),
                valid: false,
                errors: vec!["ANTHROPIC_API_KEY environment variable not set".to_string()],
                config: HashMap::new(),
            },
        }
    }

    fn validate_bedrock() -> AuthStatus {
        let mut errors = Vec::new();

        if std::env::var("CLAUDE_CODE_USE_BEDROCK").as_deref() != Ok("1") {
            errors.push("CLAUDE_CODE_USE_BEDROCK must be set to '1'".to_string());
        }
        if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
            errors.push("AWS_ACCESS_KEY_ID environment variable not set".to_string());
        }
        if std::env::var("AWS_SECRET_ACCESS_KEY").is_err() {
            errors.push("AWS_SECRET_ACCESS_KEY environment variable not set".to_string());
        }
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .ok();
        if region.is_none() {
            errors.push("AWS_REGION or AWS_DEFAULT_REGION environment variable not set".to_string());
        }

        AuthStatus {
            method: "bedrock".to_string(),
            valid: errors.is_empty(),
            errors,
            config: HashMap::from([
                ("aws_region".to_string(), serde_json::json!(region)),
            ]),
        }
    }

    fn validate_vertex() -> AuthStatus {
        let mut errors = Vec::new();

        if std::env::var("CLAUDE_CODE_USE_VERTEX").as_deref() != Ok("1") {
            errors.push("CLAUDE_CODE_USE_VERTEX must be set to '1'".to_string());
        }
        let project_id = std::env::var("ANTHROPIC_VERTEX_PROJECT_ID").ok();
        if project_id.is_none() {
            errors.push("ANTHROPIC_VERTEX_PROJECT_ID environment variable not set".to_string());
        }
        let region = std::env::var("CLOUD_ML_REGION").ok();
        if region.is_none() {
            errors.push("CLOUD_ML_REGION environment variable not set".to_string());
        }

        AuthStatus {
            method: "vertex".to_string(),
            valid: errors.is_empty(),
            errors,
            config: HashMap::from([
                ("project_id".to_string(), serde_json::json!(project_id)),
                ("region".to_string(), serde_json::json!(region)),
            ]),
        }
    }

    /// Get environment variables needed for Claude CLI subprocess.
    pub fn get_claude_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        match self.auth_method.as_str() {
            "anthropic" => {
                if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                    env.insert("ANTHROPIC_API_KEY".to_string(), key);
                }
            }
            "bedrock" => {
                env.insert("CLAUDE_CODE_USE_BEDROCK".to_string(), "1".to_string());
                for var in &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_REGION"] {
                    if let Ok(val) = std::env::var(var) {
                        env.insert(var.to_string(), val);
                    }
                }
            }
            "vertex" => {
                env.insert("CLAUDE_CODE_USE_VERTEX".to_string(), "1".to_string());
                for var in &["ANTHROPIC_VERTEX_PROJECT_ID", "CLOUD_ML_REGION", "GOOGLE_APPLICATION_CREDENTIALS"] {
                    if let Ok(val) = std::env::var(var) {
                        env.insert(var.to_string(), val);
                    }
                }
            }
            _ => {} // claude_cli: no env vars needed
        }

        // Forward Claude Code CLI env vars that control timeouts and output limits.
        // These are read by the CLI subprocess, not by the wrapper itself.
        for var in &[
            "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
            "BASH_DEFAULT_TIMEOUT_MS",
            "BASH_MAX_TIMEOUT_MS",
            "MAX_THINKING_TOKENS",
        ] {
            if let Ok(val) = std::env::var(var) {
                env.insert(var.to_string(), val);
            }
        }

        env
    }

    pub fn get_auth_info(&self) -> serde_json::Value {
        serde_json::json!({
            "method": self.auth_method,
            "status": {
                "method": self.auth_status.method,
                "valid": self.auth_status.valid,
                "errors": self.auth_status.errors,
                "config": self.auth_status.config,
            },
            "environment_variables": self.get_claude_env_vars().keys().collect::<Vec<_>>(),
        })
    }

    #[cfg(test)]
    pub fn new_test(method: &str, api_key: Option<String>) -> Self {
        Self {
            auth_method: method.to_string(),
            auth_status: AuthStatus {
                method: method.to_string(),
                valid: true,
                errors: vec![],
                config: HashMap::new(),
            },
            api_key,
            runtime_api_key: None,
        }
    }

    pub fn verify_api_key(&self, provided: Option<&str>) -> Result<(), String> {
        let active_key = self.get_api_key();

        // No key configured = allow all
        if active_key.is_none() {
            return Ok(());
        }

        match provided {
            None => Err("Missing API key".to_string()),
            Some(key) if key == active_key.unwrap() => Ok(()),
            Some(_) => Err("Invalid API key".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_api_key_no_key_configured() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", None);
        assert!(mgr.verify_api_key(None).is_ok());
        assert!(mgr.verify_api_key(Some("anything")).is_ok());
    }

    #[test]
    fn test_verify_api_key_correct() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", Some("secret123".into()));
        assert!(mgr.verify_api_key(Some("secret123")).is_ok());
    }

    #[test]
    fn test_verify_api_key_wrong() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", Some("secret123".into()));
        let err = mgr.verify_api_key(Some("wrong")).unwrap_err();
        assert!(err.contains("Invalid"));
    }

    #[test]
    fn test_verify_api_key_missing() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", Some("secret123".into()));
        let err = mgr.verify_api_key(None).unwrap_err();
        assert!(err.contains("Missing"));
    }

    #[test]
    fn test_runtime_key_overrides_env() {
        let mut mgr = ClaudeCodeAuthManager::new_test("claude_cli", Some("env_key".into()));
        mgr.set_runtime_api_key("runtime_key".into());
        assert_eq!(mgr.get_api_key(), Some("runtime_key"));
        assert!(mgr.verify_api_key(Some("runtime_key")).is_ok());
        assert!(mgr.verify_api_key(Some("env_key")).is_err());
    }

    #[test]
    fn test_get_api_key_env_only() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", Some("env_key".into()));
        assert_eq!(mgr.get_api_key(), Some("env_key"));
    }

    #[test]
    fn test_get_auth_info_structure() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", None);
        let info = mgr.get_auth_info();
        assert_eq!(info["method"], "claude_cli");
        assert!(info["status"]["valid"].as_bool().unwrap());
    }

    #[test]
    fn test_cli_auth_env_vars_empty() {
        let mgr = ClaudeCodeAuthManager::new_test("claude_cli", None);
        assert!(mgr.get_claude_env_vars().is_empty());
    }

    #[test]
    fn test_detect_cli_default() {
        // Test with a Config that has no explicit auth method set
        let config = Config {
            claude_auth_method: None,
            ..Config::from_env()
        };
        // Remove env vars that would trigger auto-detect
        std::env::remove_var("CLAUDE_CODE_USE_BEDROCK");
        std::env::remove_var("CLAUDE_CODE_USE_VERTEX");
        std::env::remove_var("ANTHROPIC_API_KEY");
        let method = ClaudeCodeAuthManager::detect_auth_method(&config);
        assert_eq!(method, "claude_cli");
    }

    #[test]
    fn test_detect_explicit_method() {
        let config = Config {
            claude_auth_method: Some("bedrock".into()),
            ..Config::from_env()
        };
        let method = ClaudeCodeAuthManager::detect_auth_method(&config);
        assert_eq!(method, "bedrock");
        std::env::remove_var("CLAUDE_AUTH_METHOD");
    }

    #[test]
    fn test_validate_cli_is_valid() {
        let status = ClaudeCodeAuthManager::validate_auth_method("claude_cli");
        assert!(status.valid);
        assert!(status.errors.is_empty());
    }

    #[test]
    fn test_validate_unknown_method() {
        let status = ClaudeCodeAuthManager::validate_auth_method("magic");
        assert!(!status.valid);
        assert!(!status.errors.is_empty());
    }
}

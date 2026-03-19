use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub claude_cli_path: String,
    pub claude_auth_method: Option<String>,
    pub api_key: Option<String>,
    pub default_model: String,
    pub max_timeout_ms: u64,
    pub max_request_size: usize,
    pub cors_origins: Vec<String>,
    pub debug_mode: bool,
    pub verbose: bool,
    pub claude_cwd: Option<String>,
    pub rate_limit_enabled: bool,
    pub rate_limit_per_minute: u32,
    pub rate_limit_chat_per_minute: u32,
    pub rate_limit_debug_per_minute: u32,
    pub rate_limit_auth_per_minute: u32,
    pub rate_limit_session_per_minute: u32,
    pub rate_limit_health_per_minute: u32,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let cors_origins = std::env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "[\"*\"]".to_string());
        let cors_origins: Vec<String> = serde_json::from_str(&cors_origins)
            .unwrap_or_else(|_| vec!["*".to_string()]);

        let debug_mode = is_truthy("DEBUG_MODE");
        let verbose = is_truthy("VERBOSE");

        Self {
            port: env_or("PORT", 8000),
            host: std::env::var("CLAUDE_WRAPPER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            claude_cli_path: std::env::var("CLAUDE_CLI_PATH").unwrap_or_else(|_| "claude".to_string()),
            claude_auth_method: std::env::var("CLAUDE_AUTH_METHOD").ok(),
            api_key: std::env::var("API_KEY").ok(),
            default_model: std::env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string()),
            max_timeout_ms: env_or("MAX_TIMEOUT", 600_000),
            max_request_size: env_or("MAX_REQUEST_SIZE", 10 * 1024 * 1024),
            cors_origins,
            debug_mode,
            verbose,
            claude_cwd: std::env::var("CLAUDE_CWD").ok(),
            rate_limit_enabled: is_truthy_default("RATE_LIMIT_ENABLED", true),
            rate_limit_per_minute: env_or("RATE_LIMIT_PER_MINUTE", 30),
            rate_limit_chat_per_minute: env_or("RATE_LIMIT_CHAT_PER_MINUTE", 10),
            rate_limit_debug_per_minute: env_or("RATE_LIMIT_DEBUG_PER_MINUTE", 2),
            rate_limit_auth_per_minute: env_or("RATE_LIMIT_AUTH_PER_MINUTE", 10),
            rate_limit_session_per_minute: env_or("RATE_LIMIT_SESSION_PER_MINUTE", 15),
            rate_limit_health_per_minute: env_or("RATE_LIMIT_HEALTH_PER_MINUTE", 30),
        }
    }

    pub fn is_debug(&self) -> bool {
        self.debug_mode || self.verbose
    }
}

fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn is_truthy(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false)
}

fn is_truthy_default(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"),
        Err(_) => default,
    }
}

pub type SharedConfig = Arc<Config>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Clear env vars that might interfere
        std::env::remove_var("PORT");
        std::env::remove_var("DEBUG_MODE");
        std::env::remove_var("VERBOSE");
        std::env::remove_var("CLAUDE_CWD");
        std::env::remove_var("API_KEY");
        std::env::remove_var("CLAUDE_AUTH_METHOD");

        let config = Config::from_env();
        assert_eq!(config.port, 8000);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.claude_cli_path, "claude");
        assert_eq!(config.max_timeout_ms, 600_000);
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert!(!config.debug_mode);
        assert!(!config.verbose);
        assert!(config.api_key.is_none());
        assert!(config.claude_cwd.is_none());
    }

    #[test]
    fn test_config_is_debug() {
        let mut config = Config::from_env();
        config.debug_mode = false;
        config.verbose = false;
        assert!(!config.is_debug());

        config.debug_mode = true;
        assert!(config.is_debug());

        config.debug_mode = false;
        config.verbose = true;
        assert!(config.is_debug());
    }

    #[test]
    fn test_env_or_with_default() {
        std::env::remove_var("__TEST_ENV_OR_MISSING");
        assert_eq!(env_or::<u16>("__TEST_ENV_OR_MISSING", 42), 42);
    }

    #[test]
    fn test_env_or_with_value() {
        std::env::set_var("__TEST_ENV_OR_SET", "99");
        assert_eq!(env_or::<u16>("__TEST_ENV_OR_SET", 42), 99);
        std::env::remove_var("__TEST_ENV_OR_SET");
    }

    #[test]
    fn test_env_or_invalid_parse() {
        std::env::set_var("__TEST_ENV_OR_BAD", "not_a_number");
        assert_eq!(env_or::<u16>("__TEST_ENV_OR_BAD", 42), 42);
        std::env::remove_var("__TEST_ENV_OR_BAD");
    }

    #[test]
    fn test_is_truthy() {
        std::env::remove_var("__TEST_TRUTHY");
        assert!(!is_truthy("__TEST_TRUTHY"));

        for val in &["true", "1", "yes", "on", "TRUE", "Yes", "ON"] {
            std::env::set_var("__TEST_TRUTHY", val);
            assert!(is_truthy("__TEST_TRUTHY"), "Expected truthy for '{val}'");
        }

        for val in &["false", "0", "no", "off", "random"] {
            std::env::set_var("__TEST_TRUTHY", val);
            assert!(!is_truthy("__TEST_TRUTHY"), "Expected falsy for '{val}'");
        }
        std::env::remove_var("__TEST_TRUTHY");
    }

    #[test]
    fn test_is_truthy_default() {
        std::env::remove_var("__TEST_TRUTHY_DEF");
        assert!(is_truthy_default("__TEST_TRUTHY_DEF", true));
        assert!(!is_truthy_default("__TEST_TRUTHY_DEF", false));

        std::env::set_var("__TEST_TRUTHY_DEF", "true");
        assert!(is_truthy_default("__TEST_TRUTHY_DEF", false));

        std::env::set_var("__TEST_TRUTHY_DEF", "false");
        assert!(!is_truthy_default("__TEST_TRUTHY_DEF", true));
        std::env::remove_var("__TEST_TRUTHY_DEF");
    }

    #[test]
    fn test_cors_origins_parsing() {
        std::env::set_var("CORS_ORIGINS", r#"["http://localhost:3000","http://example.com"]"#);
        let config = Config::from_env();
        assert_eq!(config.cors_origins.len(), 2);
        assert!(config.cors_origins.contains(&"http://localhost:3000".to_string()));
        std::env::remove_var("CORS_ORIGINS");
    }
}

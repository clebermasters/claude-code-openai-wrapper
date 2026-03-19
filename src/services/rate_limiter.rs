use governor::{Quota, RateLimiter as GovRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::config::Config;

/// Per-endpoint rate limiter using governor.
#[derive(Clone)]
pub struct RateLimiterSet {
    pub chat: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub health: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub models: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub debug: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub auth: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub session: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub general: Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    pub enabled: bool,
}

fn make_limiter(per_minute: u32) -> Arc<GovRateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>> {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute.max(1)).unwrap());
    Arc::new(GovRateLimiter::direct(quota))
}

impl RateLimiterSet {
    pub fn new(config: &Config) -> Self {
        Self {
            chat: make_limiter(config.rate_limit_chat_per_minute),
            health: make_limiter(config.rate_limit_health_per_minute),
            models: make_limiter(100),
            debug: make_limiter(config.rate_limit_debug_per_minute),
            auth: make_limiter(config.rate_limit_auth_per_minute),
            session: make_limiter(config.rate_limit_session_per_minute),
            general: make_limiter(config.rate_limit_per_minute),
            enabled: config.rate_limit_enabled,
        }
    }

    pub fn check(&self, category: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let limiter = match category {
            "chat" => &self.chat,
            "health" => &self.health,
            "models" => &self.models,
            "debug" => &self.debug,
            "auth" => &self.auth,
            "session" => &self.session,
            _ => &self.general,
        };

        limiter.check().map_err(|_| {
            format!("Rate limit exceeded for '{category}'. Please retry later.")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            port: 8000,
            host: "0.0.0.0".into(),
            claude_cli_path: "claude".into(),
            claude_auth_method: None,
            api_key: None,
            default_model: "opus".into(),
            max_timeout_ms: 60000,
            max_request_size: 1024,
            cors_origins: vec!["*".into()],
            debug_mode: false,
            verbose: false,
            claude_cwd: None,
            rate_limit_enabled: true,
            rate_limit_per_minute: 60,
            rate_limit_chat_per_minute: 10,
            rate_limit_debug_per_minute: 2,
            rate_limit_auth_per_minute: 10,
            rate_limit_session_per_minute: 15,
            rate_limit_health_per_minute: 30,
        }
    }

    #[test]
    fn test_rate_limiter_enabled_allows_first() {
        let rl = RateLimiterSet::new(&test_config());
        assert!(rl.check("chat").is_ok());
        assert!(rl.check("health").is_ok());
        assert!(rl.check("models").is_ok());
        assert!(rl.check("unknown_category").is_ok()); // falls to general
    }

    #[test]
    fn test_rate_limiter_disabled_always_ok() {
        let mut cfg = test_config();
        cfg.rate_limit_enabled = false;
        let rl = RateLimiterSet::new(&cfg);
        // Even with very low limits, disabled should pass
        for _ in 0..100 {
            assert!(rl.check("chat").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_exceeds() {
        let mut cfg = test_config();
        cfg.rate_limit_debug_per_minute = 1; // 1 per minute
        let rl = RateLimiterSet::new(&cfg);
        assert!(rl.check("debug").is_ok()); // first one OK
        // Second should fail (1/min already consumed)
        let result = rl.check("debug");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Rate limit exceeded"));
    }
}

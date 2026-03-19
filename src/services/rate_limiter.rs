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

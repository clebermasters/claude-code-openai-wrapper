mod config;
mod constants;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use config::Config;
use services::auth::ClaudeCodeAuthManager;
use services::claude_cli::ClaudeCli;
use services::mcp_client::MCPClient;
use services::rate_limiter::RateLimiterSet;
use services::session_manager::SessionManager;
use services::tool_manager::ToolManager;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub claude_cli: ClaudeCli,
    pub session_manager: SessionManager,
    pub auth_manager: ClaudeCodeAuthManager,
    pub tool_manager: ToolManager,
    pub mcp_client: MCPClient,
    pub rate_limiters: RateLimiterSet,
}

#[tokio::main]
async fn main() {
    // Load config
    let config = Config::from_env();
    let config = Arc::new(config);

    // Init tracing
    let log_level = if config.is_debug() { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    info!("Claude Code OpenAI Wrapper v{} (Rust)", constants::VERSION);

    // Init auth manager
    let auth_manager = ClaudeCodeAuthManager::new(&config);

    if !auth_manager.auth_status.valid {
        warn!("Claude Code authentication issues: {:?}", auth_manager.auth_status.errors);
    } else {
        info!("Claude Code auth validated: {}", auth_manager.auth_method);
    }

    // Init Claude CLI
    let auth_env_vars = auth_manager.get_claude_env_vars();
    let claude_cli = match ClaudeCli::new(config.clone(), auth_env_vars) {
        Ok(cli) => cli,
        Err(e) => {
            tracing::error!("Failed to initialize Claude CLI: {e}");
            tracing::error!("The server will start, but requests will fail.");
            ClaudeCli::new(
                config.clone(),
                auth_manager.get_claude_env_vars(),
            )
            .expect("Failed to create fallback CLI instance")
        }
    };

    // Verify CLI (non-blocking)
    let cli_clone = claude_cli.clone();
    tokio::spawn(async move {
        match tokio::time::timeout(std::time::Duration::from_secs(30), cli_clone.verify_cli()).await
        {
            Ok(true) => info!("Claude CLI verified successfully"),
            Ok(false) => warn!("Claude CLI verification returned false"),
            Err(_) => warn!("Claude CLI verification timed out (30s)"),
        }
    });

    // Init session manager with cleanup task
    let session_manager = SessionManager::default();
    session_manager.start_cleanup_task();

    // Init other services
    let tool_manager = ToolManager::new();
    let mcp_client = MCPClient::new();
    let rate_limiters = RateLimiterSet::new(&config);

    // Build app state
    let state = AppState {
        config: config.clone(),
        claude_cli,
        session_manager,
        auth_manager,
        tool_manager,
        mcp_client,
        rate_limiters,
    };

    // Build router
    let app = Router::new()
        .route("/", get(handlers::landing::root))
        .route("/health", get(handlers::health::health_check))
        .route("/version", get(handlers::health::version_info))
        .route("/v1/chat/completions", post(handlers::chat::chat_completions))
        .route("/v1/messages", post(handlers::messages::anthropic_messages))
        .route("/v1/models", get(handlers::models::list_models))
        .route("/v1/auth/status", get(handlers::auth_status::get_status))
        .route("/v1/sessions", get(handlers::sessions::list))
        .route("/v1/sessions/stats", get(handlers::sessions::stats))
        .route(
            "/v1/sessions/{id}",
            get(handlers::sessions::get).delete(handlers::sessions::delete),
        )
        .route("/v1/tools", get(handlers::tools::list_tools))
        .route(
            "/v1/tools/config",
            get(handlers::tools::get_config).post(handlers::tools::update_config),
        )
        .route("/v1/tools/stats", get(handlers::tools::get_stats))
        .route(
            "/v1/mcp/servers",
            get(handlers::mcp::list_servers).post(handlers::mcp::register_server),
        )
        .route("/v1/mcp/connect", post(handlers::mcp::connect_server))
        .route("/v1/mcp/disconnect", post(handlers::mcp::disconnect_server))
        .route("/v1/mcp/stats", get(handlers::mcp::get_stats))
        .route("/v1/debug/request", post(handlers::debug::debug_request))
        .route("/v1/compatibility", post(handlers::compatibility::check))
        .with_state(state);

    // CORS
    let cors = if config.cors_origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
    };
    let app = app.layer(cors);

    // Serve
    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting server on {addr}");
    info!("Landing page: http://localhost:{}", config.port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {addr}: {e}"));

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

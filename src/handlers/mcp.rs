use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;

use crate::error::AppError;
use crate::models::mcp::*;
use crate::services::mcp_client::MCPServerConfig;
use crate::AppState;

/// GET /v1/mcp/servers
pub async fn list_servers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    if !state.mcp_client.is_available() {
        return Err(AppError::ServiceUnavailable(
            "MCP SDK not available in Rust build. Server registration works but connections are not supported.".to_string(),
        ));
    }

    let servers = state.mcp_client.list_servers().await;
    let responses: Vec<MCPServerInfoResponse> = servers
        .iter()
        .map(|s| MCPServerInfoResponse {
            name: s.name.clone(),
            command: s.command.clone(),
            args: s.args.clone(),
            description: s.description.clone(),
            enabled: s.enabled,
            connected: false,
            tools_count: 0,
            resources_count: 0,
            prompts_count: 0,
        })
        .collect();

    let total = responses.len();
    Ok(Json(serde_json::to_value(MCPServersListResponse { servers: responses, total }).unwrap()))
}

/// POST /v1/mcp/servers
pub async fn register_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MCPServerConfigRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    body.validate().map_err(AppError::BadRequest)?;

    state.mcp_client.register_server(MCPServerConfig {
        name: body.name.clone(),
        command: body.command,
        args: body.args,
        env: body.env,
        description: body.description,
        enabled: body.enabled,
    }).await;

    Ok(Json(serde_json::json!({
        "message": format!("MCP server '{}' registered successfully", body.name)
    })))
}

/// POST /v1/mcp/connect
pub async fn connect_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MCPConnectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    body.validate().map_err(AppError::BadRequest)?;

    Err(AppError::ServiceUnavailable(
        "MCP connections are not supported in Rust build".to_string(),
    ))
}

/// POST /v1/mcp/disconnect
pub async fn disconnect_server(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MCPConnectionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    body.validate().map_err(AppError::BadRequest)?;

    Err(AppError::ServiceUnavailable(
        "MCP connections are not supported in Rust build".to_string(),
    ))
}

/// GET /v1/mcp/stats
pub async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    Ok(Json(state.mcp_client.get_stats().await))
}

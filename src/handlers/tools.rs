use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;

use crate::constants::CLAUDE_TOOLS;
use crate::error::AppError;
use crate::models::tool::*;
use crate::AppState;

/// GET /v1/tools
pub async fn list_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ToolListResponse>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    let tools = state.tool_manager.list_all_tools();
    let total = tools.len();
    Ok(Json(ToolListResponse { tools, total }))
}

#[derive(serde::Deserialize)]
pub struct ConfigQuery {
    pub session_id: Option<String>,
}

/// GET /v1/tools/config
pub async fn get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ConfigQuery>,
) -> Result<Json<ToolConfigurationResponse>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    let config = state.tool_manager.get_effective_config(query.session_id.as_deref()).await;
    let effective = state.tool_manager.get_effective_tools(query.session_id.as_deref()).await;

    Ok(Json(ToolConfigurationResponse {
        allowed_tools: config.allowed_tools,
        disallowed_tools: config.disallowed_tools,
        effective_tools: effective,
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

/// POST /v1/tools/config
pub async fn update_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ToolConfigurationRequest>,
) -> Result<Json<ToolConfigurationResponse>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    // Validate tool names
    let mut all_names = Vec::new();
    if let Some(ref a) = request.allowed_tools {
        all_names.extend(a.clone());
    }
    if let Some(ref d) = request.disallowed_tools {
        all_names.extend(d.clone());
    }

    if !all_names.is_empty() {
        let validation = state.tool_manager.validate_tools(&all_names);
        let invalid: Vec<String> = validation
            .iter()
            .filter(|(_, &v)| !v)
            .map(|(k, _)| k.clone())
            .collect();
        if !invalid.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Invalid tool names: {}. Valid tools: {}",
                invalid.join(", "),
                CLAUDE_TOOLS.join(", "),
            )));
        }
    }

    let config = if let Some(ref sid) = request.session_id {
        state.tool_manager.set_session_config(sid, request.allowed_tools, request.disallowed_tools).await
    } else {
        state.tool_manager.update_global_config(request.allowed_tools, request.disallowed_tools).await
    };

    let effective = state.tool_manager.get_effective_tools(request.session_id.as_deref()).await;

    Ok(Json(ToolConfigurationResponse {
        allowed_tools: config.allowed_tools,
        disallowed_tools: config.disallowed_tools,
        effective_tools: effective,
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

/// GET /v1/tools/stats
pub async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("general").map_err(AppError::RateLimited)?;

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    Ok(Json(state.tool_manager.get_stats().await))
}

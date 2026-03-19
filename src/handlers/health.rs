use axum::extract::State;
use axum::Json;

use crate::constants::VERSION;
use crate::error::AppError;
use crate::AppState;

/// GET /health
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("health").map_err(AppError::RateLimited)?;

    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "claude-code-openai-wrapper",
    })))
}

/// GET /version
pub async fn version_info(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("health").map_err(AppError::RateLimited)?;

    Ok(Json(serde_json::json!({
        "version": VERSION,
        "service": "claude-code-openai-wrapper",
        "api_version": "v1",
    })))
}

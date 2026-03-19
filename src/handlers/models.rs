use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;

use crate::constants::CLAUDE_MODELS;
use crate::error::AppError;
use crate::AppState;

/// GET /v1/models
pub async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("models").map_err(AppError::RateLimited)?;

    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    let models: Vec<serde_json::Value> = CLAUDE_MODELS
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "object": "model",
                "owned_by": "anthropic",
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": models,
    })))
}

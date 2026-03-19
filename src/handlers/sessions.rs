use axum::extract::{Path, State};
use axum::Json;

use crate::error::AppError;
use crate::AppState;

/// GET /v1/sessions
pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("session").map_err(AppError::RateLimited)?;

    let response = state.session_manager.list_sessions().await;
    Ok(Json(serde_json::to_value(response).unwrap()))
}

/// GET /v1/sessions/stats
pub async fn stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("session").map_err(AppError::RateLimited)?;

    let stats = state.session_manager.get_stats().await;
    Ok(Json(serde_json::json!({
        "session_stats": stats,
        "cleanup_interval_secs": state.session_manager.cleanup_interval_secs,
        "default_ttl_hours": state.session_manager.default_ttl_hours,
    })))
}

/// GET /v1/sessions/:id
pub async fn get(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("session").map_err(AppError::RateLimited)?;

    match state.session_manager.get_session(&session_id).await {
        Some(session) => Ok(Json(serde_json::to_value(session.to_session_info()).unwrap())),
        None => Err(AppError::NotFound("Session not found".to_string())),
    }
}

/// DELETE /v1/sessions/:id
pub async fn delete(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("session").map_err(AppError::RateLimited)?;

    if state.session_manager.delete_session(&session_id).await {
        Ok(Json(serde_json::json!({
            "message": format!("Session {session_id} deleted successfully")
        })))
    } else {
        Err(AppError::NotFound("Session not found".to_string()))
    }
}

use axum::extract::State;
use axum::Json;

use crate::error::AppError;
use crate::AppState;

/// GET /v1/auth/status
pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("auth").map_err(AppError::RateLimited)?;

    let auth_info = state.auth_manager.get_auth_info();

    Ok(Json(serde_json::json!({
        "claude_code_auth": auth_info,
        "server_info": {
            "api_key_required": state.auth_manager.get_api_key().is_some(),
            "api_key_source": if state.config.api_key.is_some() {
                "environment"
            } else if state.auth_manager.runtime_api_key.is_some() {
                "runtime"
            } else {
                "none"
            },
            "version": crate::constants::VERSION,
        },
    })))
}

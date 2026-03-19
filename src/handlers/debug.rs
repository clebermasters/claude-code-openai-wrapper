use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;

use crate::error::AppError;
use crate::models::openai::ChatCompletionRequest;
use crate::AppState;

/// POST /v1/debug/request
pub async fn debug_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    state.rate_limiters.check("debug").map_err(AppError::RateLimited)?;

    let raw_body = String::from_utf8_lossy(&body).to_string();

    let parsed_body: Option<serde_json::Value> = serde_json::from_str(&raw_body).ok();
    let json_error: Option<String> = if parsed_body.is_none() && !raw_body.is_empty() {
        Some(serde_json::from_str::<serde_json::Value>(&raw_body).unwrap_err().to_string())
    } else {
        None
    };

    let validation_result = if let Some(ref parsed) = parsed_body {
        match serde_json::from_value::<ChatCompletionRequest>(parsed.clone()) {
            Ok(req) => serde_json::json!({"valid": true}),
            Err(e) => serde_json::json!({"valid": false, "errors": [e.to_string()]}),
        }
    } else {
        serde_json::json!({"valid": false, "errors": ["Could not parse JSON"]})
    };

    let header_map: std::collections::HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    Ok(Json(serde_json::json!({
        "debug_info": {
            "headers": header_map,
            "raw_body": raw_body,
            "json_parse_error": json_error,
            "parsed_body": parsed_body,
            "validation_result": validation_result,
            "debug_mode_enabled": state.config.is_debug(),
            "example_valid_request": {
                "model": "claude-sonnet-4-5-20250929",
                "messages": [{"role": "user", "content": "Hello, world!"}],
                "stream": false,
            },
        }
    })))
}

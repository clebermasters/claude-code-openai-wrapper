use axum::Json;

use crate::error::AppError;
use crate::models::openai::ChatCompletionRequest;
use crate::services::parameter_validator::CompatibilityReporter;

/// POST /v1/compatibility
pub async fn check(
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let report = CompatibilityReporter::generate_report(&request);

    Ok(Json(serde_json::json!({
        "compatibility_report": report,
        "claude_agent_sdk_options": {
            "supported": [
                "model", "system_prompt", "max_turns", "allowed_tools",
                "disallowed_tools", "permission_mode", "max_thinking_tokens",
                "continue_conversation", "resume", "cwd",
            ],
            "custom_headers": [
                "X-Claude-Max-Turns",
                "X-Claude-Allowed-Tools",
                "X-Claude-Disallowed-Tools",
                "X-Claude-Permission-Mode",
                "X-Claude-Max-Thinking-Tokens",
            ],
        },
    })))
}

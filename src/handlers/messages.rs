use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use tracing::{error, info};

use crate::error::AppError;
use crate::models::anthropic::*;
use crate::services::content_filter;
use crate::services::message_adapter;
use crate::AppState;

/// POST /v1/messages (Anthropic format)
pub async fn anthropic_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AnthropicMessagesRequest>,
) -> Result<Json<AnthropicMessagesResponse>, AppError> {
    // Rate limit
    state.rate_limiters.check("chat").map_err(AppError::RateLimited)?;

    // Verify API key
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    state.auth_manager.verify_api_key(bearer).map_err(AppError::Unauthorized)?;

    // Verify Claude auth
    if !state.auth_manager.auth_status.valid {
        return Err(AppError::ServiceUnavailable(
            "Claude Code authentication failed".to_string(),
        ));
    }

    info!("Anthropic Messages API request: model={}", request.model);

    // Convert to internal format
    let messages = request.to_openai_messages();

    // Build prompt
    let prompt_parts: Vec<String> = messages
        .iter()
        .map(|msg| match msg.role.as_str() {
            "user" => msg.content.clone(),
            "assistant" => format!("Assistant: {}", msg.content),
            _ => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    let prompt = content_filter::filter_content(&prompt_parts.join("\n\n"));
    let system_prompt = request.system.as_deref().map(content_filter::filter_content);

    // Run Claude CLI - tools enabled by default for Anthropic SDK clients
    let allowed: Vec<String> = crate::constants::DEFAULT_ALLOWED_TOOLS
        .iter()
        .map(|s| s.to_string())
        .collect();

    let result = state
        .claude_cli
        .run_completion(
            &prompt,
            system_prompt.as_deref(),
            Some(&request.model),
            Some(&allowed),
            None,
            Some("bypassPermissions"),
        )
        .await
        .map_err(AppError::Internal)?;

    if result.text.is_empty() {
        return Err(AppError::Internal("No response from Claude Code".to_string()));
    }

    let assistant_content = content_filter::filter_content(&result.text);
    let input_tokens = message_adapter::estimate_tokens(&prompt);
    let output_tokens = message_adapter::estimate_tokens(&assistant_content);

    Ok(Json(AnthropicMessagesResponse::new(
        request.model.clone(),
        assistant_content,
        input_tokens,
        output_tokens,
    )))
}

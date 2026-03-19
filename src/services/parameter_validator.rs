use std::collections::HashMap;
use tracing::{info, warn};

use crate::constants::CLAUDE_MODELS;
use crate::models::openai::ChatCompletionRequest;

pub struct ParameterValidator;

const VALID_PERMISSION_MODES: &[&str] = &["default", "acceptEdits", "bypassPermissions", "plan"];

impl ParameterValidator {
    pub fn validate_model(model: &str) -> bool {
        if !CLAUDE_MODELS.contains(&model) {
            warn!(
                "Model '{}' is not in the known supported models list. It will still be attempted but may fail.",
                model
            );
        }
        true
    }

    pub fn validate_permission_mode(mode: &str) -> bool {
        if !VALID_PERMISSION_MODES.contains(&mode) {
            warn!("Invalid permission_mode '{}'. Valid options: {:?}", mode, VALID_PERMISSION_MODES);
            return false;
        }
        true
    }

    pub fn validate_tools(tools: &[String]) -> bool {
        tools.iter().all(|t| !t.trim().is_empty())
    }

    /// Extract Claude-Code-specific parameters from custom HTTP headers.
    pub fn extract_claude_headers(headers: &axum::http::HeaderMap) -> HashMap<String, serde_json::Value> {
        let mut options = HashMap::new();

        if let Some(val) = headers.get("x-claude-max-turns").and_then(|v| v.to_str().ok()) {
            if let Ok(n) = val.parse::<u32>() {
                options.insert("max_turns".to_string(), serde_json::json!(n));
            } else {
                warn!("Invalid X-Claude-Max-Turns header: {val}");
            }
        }

        if let Some(val) = headers.get("x-claude-allowed-tools").and_then(|v| v.to_str().ok()) {
            let tools: Vec<String> = val.split(',').map(|s| s.trim().to_string()).collect();
            if !tools.is_empty() {
                options.insert("allowed_tools".to_string(), serde_json::json!(tools));
            }
        }

        if let Some(val) = headers.get("x-claude-disallowed-tools").and_then(|v| v.to_str().ok()) {
            let tools: Vec<String> = val.split(',').map(|s| s.trim().to_string()).collect();
            if !tools.is_empty() {
                options.insert("disallowed_tools".to_string(), serde_json::json!(tools));
            }
        }

        if let Some(val) = headers.get("x-claude-permission-mode").and_then(|v| v.to_str().ok()) {
            options.insert("permission_mode".to_string(), serde_json::json!(val));
        }

        if let Some(val) = headers.get("x-claude-max-thinking-tokens").and_then(|v| v.to_str().ok()) {
            if let Ok(n) = val.parse::<u32>() {
                options.insert("max_thinking_tokens".to_string(), serde_json::json!(n));
            } else {
                warn!("Invalid X-Claude-Max-Thinking-Tokens header: {val}");
            }
        }

        options
    }
}

pub struct CompatibilityReporter;

impl CompatibilityReporter {
    pub fn generate_report(request: &ChatCompletionRequest) -> serde_json::Value {
        let mut supported = vec!["model", "messages"];
        let mut unsupported = Vec::new();
        let mut suggestions = Vec::new();

        if request.stream.is_some() {
            supported.push("stream");
        }
        if request.user.is_some() {
            supported.push("user (for logging)");
        }

        if request.temperature != Some(1.0) {
            unsupported.push("temperature");
            suggestions.push("Claude Code SDK does not support temperature control directly.");
        }
        if request.top_p != Some(1.0) {
            unsupported.push("top_p");
        }
        if request.max_tokens.is_some() {
            unsupported.push("max_tokens");
            suggestions.push("Use max_turns or max_thinking_tokens instead.");
        }
        if request.n.unwrap_or(1) > 1 {
            unsupported.push("n");
        }
        if request.stop.is_some() {
            unsupported.push("stop");
        }
        if request.presence_penalty.unwrap_or(0.0) != 0.0
            || request.frequency_penalty.unwrap_or(0.0) != 0.0
        {
            unsupported.push("presence_penalty/frequency_penalty");
        }
        if request.logit_bias.is_some() {
            unsupported.push("logit_bias");
        }

        serde_json::json!({
            "supported_parameters": supported,
            "unsupported_parameters": unsupported,
            "suggestions": suggestions,
        })
    }
}

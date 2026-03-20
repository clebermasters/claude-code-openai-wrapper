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

        if let Some(val) = headers.get("x-claude-include-thinking").and_then(|v| v.to_str().ok()) {
            let include = matches!(val.to_lowercase().as_str(), "true" | "1" | "yes");
            options.insert("include_thinking".to_string(), serde_json::json!(include));
        }

        if let Some(val) = headers.get("x-claude-effort").and_then(|v| v.to_str().ok()) {
            let effort = val.to_lowercase();
            if ["low", "medium", "high", "max"].contains(&effort.as_str()) {
                options.insert("effort".to_string(), serde_json::json!(effort));
            } else {
                warn!("Invalid X-Claude-Effort header: {val}. Valid: low, medium, high, max");
            }
        }

        if let Some(val) = headers.get("x-claude-max-budget-usd").and_then(|v| v.to_str().ok()) {
            match val.parse::<f64>() {
                Ok(n) if n > 0.0 => {
                    options.insert("max_budget_usd".to_string(), serde_json::json!(n));
                }
                Ok(n) => {
                    warn!("X-Claude-Max-Budget-Usd must be positive, got: {n}");
                }
                Err(_) => {
                    warn!("Invalid X-Claude-Max-Budget-Usd header: {val}");
                }
            }
        }

        if let Some(val) = headers.get("x-claude-fallback-model").and_then(|v| v.to_str().ok()) {
            let model = val.trim();
            if !model.is_empty() {
                options.insert("fallback_model".to_string(), serde_json::json!(model));
            }
        }

        if let Some(val) = headers.get("x-claude-append-system-prompt").and_then(|v| v.to_str().ok()) {
            if !val.is_empty() {
                options.insert("append_system_prompt".to_string(), serde_json::json!(val));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::openai::ChatCompletionRequest;

    fn make_req(json: &str) -> ChatCompletionRequest {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn test_validate_model_known() {
        assert!(ParameterValidator::validate_model("claude-sonnet-4-5-20250929"));
    }

    #[test]
    fn test_validate_model_unknown_still_passes() {
        // Graceful degradation: unknown models still return true
        assert!(ParameterValidator::validate_model("fake-model"));
    }

    #[test]
    fn test_validate_permission_mode_valid() {
        assert!(ParameterValidator::validate_permission_mode("bypassPermissions"));
        assert!(ParameterValidator::validate_permission_mode("default"));
        assert!(ParameterValidator::validate_permission_mode("plan"));
    }

    #[test]
    fn test_validate_permission_mode_invalid() {
        assert!(!ParameterValidator::validate_permission_mode("invalid"));
    }

    #[test]
    fn test_validate_tools_valid() {
        assert!(ParameterValidator::validate_tools(&["Read".into(), "Write".into()]));
    }

    #[test]
    fn test_validate_tools_empty_string() {
        assert!(!ParameterValidator::validate_tools(&["".into()]));
    }

    #[test]
    fn test_extract_claude_headers_max_turns() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-turns", "5".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["max_turns"], 5);
    }

    #[test]
    fn test_extract_claude_headers_allowed_tools() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-allowed-tools", "Read,Write,Bash".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        let tools = opts["allowed_tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_extract_claude_headers_permission_mode() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-permission-mode", "bypassPermissions".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["permission_mode"], "bypassPermissions");
    }

    #[test]
    fn test_extract_claude_headers_empty() {
        let headers = axum::http::HeaderMap::new();
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(opts.is_empty());
    }

    #[test]
    fn test_extract_claude_headers_invalid_int() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-turns", "abc".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("max_turns"));
    }

    #[test]
    fn test_extract_claude_headers_include_thinking_true() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-include-thinking", "true".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["include_thinking"], true);
    }

    #[test]
    fn test_extract_claude_headers_include_thinking_false() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-include-thinking", "false".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["include_thinking"], false);
    }

    #[test]
    fn test_extract_claude_headers_effort() {
        for level in &["low", "medium", "high", "max"] {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("x-claude-effort", level.parse().unwrap());
            let opts = ParameterValidator::extract_claude_headers(&headers);
            assert_eq!(opts["effort"], *level);
        }
    }

    #[test]
    fn test_extract_claude_headers_effort_invalid() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-effort", "extreme".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("effort"));
    }

    #[test]
    fn test_extract_claude_headers_max_budget_usd() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-budget-usd", "5.50".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["max_budget_usd"], 5.5);
    }

    #[test]
    fn test_extract_claude_headers_max_budget_usd_zero_rejected() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-budget-usd", "0".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("max_budget_usd"));
    }

    #[test]
    fn test_extract_claude_headers_max_budget_usd_negative_rejected() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-budget-usd", "-1.0".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("max_budget_usd"));
    }

    #[test]
    fn test_extract_claude_headers_fallback_model() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-fallback-model", "claude-haiku-4-5-20251001".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["fallback_model"], "claude-haiku-4-5-20251001");
    }

    #[test]
    fn test_extract_claude_headers_append_system_prompt() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-append-system-prompt", "Always respond in JSON".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert_eq!(opts["append_system_prompt"], "Always respond in JSON");
    }

    #[test]
    fn test_compatibility_report_defaults() {
        let req = make_req(r#"{"messages":[{"role":"user","content":"hi"}]}"#);
        let report = CompatibilityReporter::generate_report(&req);
        let supported = report["supported_parameters"].as_array().unwrap();
        assert!(supported.iter().any(|v| v == "model"));
        assert!(supported.iter().any(|v| v == "messages"));
        let unsupported = report["unsupported_parameters"].as_array().unwrap();
        assert!(unsupported.is_empty());
    }

    #[test]
    fn test_compatibility_report_unsupported() {
        let req = make_req(r#"{"messages":[{"role":"user","content":"hi"}],"temperature":0.5,"stop":"END","logit_bias":{"1":0.5}}"#);
        let report = CompatibilityReporter::generate_report(&req);
        let unsupported = report["unsupported_parameters"].as_array().unwrap();
        assert!(unsupported.iter().any(|v| v == "temperature"));
        assert!(unsupported.iter().any(|v| v == "stop"));
        assert!(unsupported.iter().any(|v| v == "logit_bias"));
    }

    #[test]
    fn test_extract_claude_headers_fallback_model_empty_rejected() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-fallback-model", "".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("fallback_model"));
    }

    #[test]
    fn test_extract_claude_headers_append_system_prompt_empty_rejected() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-append-system-prompt", "".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("append_system_prompt"));
    }

    #[test]
    fn test_extract_claude_headers_max_budget_usd_non_numeric() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-claude-max-budget-usd", "abc".parse().unwrap());
        let opts = ParameterValidator::extract_claude_headers(&headers);
        assert!(!opts.contains_key("max_budget_usd"));
    }
}

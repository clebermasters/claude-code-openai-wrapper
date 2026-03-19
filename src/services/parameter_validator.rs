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
}

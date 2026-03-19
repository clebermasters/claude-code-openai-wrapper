use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Deserialize content that can be either a string or array of content parts.
/// Always normalizes to a String.
fn deserialize_content<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ContentValue {
        Text(String),
        Parts(Vec<ContentPart>),
    }

    match ContentValue::deserialize(deserializer)? {
        ContentValue::Text(s) => Ok(s),
        ContentValue::Parts(parts) => {
            let texts: Vec<&str> = parts
                .iter()
                .filter(|p| p.r#type == "text")
                .map(|p| p.text.as_str())
                .collect();
            Ok(texts.join("\n"))
        }
    }
}

/// Deserialize stop that can be a string or array of strings.
fn deserialize_stop<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StopValue {
        Single(String),
        Multiple(Vec<String>),
    }

    let opt: Option<StopValue> = Option::deserialize(deserializer)?;
    Ok(opt.map(|v| match v {
        StopValue::Single(s) => vec![s],
        StopValue::Multiple(v) => v,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(deserialize_with = "deserialize_content")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    #[serde(default)]
    pub include_usage: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionRequest {
    #[serde(default = "default_model")]
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default = "default_temperature")]
    pub temperature: Option<f64>,
    #[serde(default = "default_top_p")]
    pub top_p: Option<f64>,
    #[serde(default = "default_n")]
    pub n: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_stop")]
    pub stop: Option<Vec<String>>,
    pub max_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    #[serde(default)]
    pub presence_penalty: Option<f64>,
    #[serde(default)]
    pub frequency_penalty: Option<f64>,
    pub logit_bias: Option<HashMap<String, f64>>,
    pub user: Option<String>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub enable_tools: Option<bool>,
    pub stream_options: Option<StreamOptions>,
}

fn default_model() -> String {
    std::env::var("DEFAULT_MODEL").unwrap_or_else(|_| "claude-sonnet-4-5-20250929".to_string())
}

fn default_temperature() -> Option<f64> {
    Some(1.0)
}

fn default_top_p() -> Option<f64> {
    Some(1.0)
}

fn default_n() -> Option<u32> {
    Some(1)
}

impl ChatCompletionRequest {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(n) = self.n {
            if n > 1 {
                return Err("Claude Code SDK does not support multiple choices (n > 1). Only single response generation is supported.".to_string());
            }
        }
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err("temperature must be between 0 and 2".to_string());
            }
        }
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err("top_p must be between 0 and 1".to_string());
            }
        }
        Ok(())
    }

    pub fn get_sampling_instructions(&self) -> Option<String> {
        let mut instructions = Vec::new();

        if let Some(temp) = self.temperature {
            if temp != 1.0 {
                if temp < 0.3 {
                    instructions.push("Be highly focused and deterministic in your responses. Choose the most likely and predictable options.");
                } else if temp < 0.7 {
                    instructions.push("Be somewhat focused and consistent in your responses, preferring reliable and expected solutions.");
                } else if temp > 1.5 {
                    instructions.push("Be highly creative and exploratory in your responses. Consider unusual and diverse approaches.");
                } else if temp > 1.0 {
                    instructions.push("Be creative and varied in your responses, exploring different approaches and possibilities.");
                }
            }
        }

        if let Some(top_p) = self.top_p {
            if top_p < 1.0 {
                if top_p < 0.5 {
                    instructions.push("Focus on the most probable and mainstream solutions, avoiding less likely alternatives.");
                } else if top_p < 0.9 {
                    instructions.push("Prefer well-established and common approaches over unusual ones.");
                }
            }
        }

        if instructions.is_empty() {
            None
        } else {
            Some(instructions.join(" "))
        }
    }

    pub fn to_claude_options(&self) -> HashMap<String, serde_json::Value> {
        let mut options = HashMap::new();

        options.insert("model".to_string(), serde_json::Value::String(self.model.clone()));

        let max_val = self.max_completion_tokens.or(self.max_tokens);
        if let Some(max) = max_val {
            options.insert("max_thinking_tokens".to_string(), serde_json::json!(max));
        }

        options
    }

    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }

    pub fn tools_enabled(&self) -> bool {
        self.enable_tools.unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionResponse {
    pub fn new(model: String, content: String, usage: Option<Usage>) -> Self {
        Self {
            id: format!("chatcmpl-{}", &Uuid::new_v4().to_string()[..8]),
            object: "chat.completion".to_string(),
            created: Utc::now().timestamp(),
            model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content,
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage,
            system_fingerprint: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatCompletionStreamResponse {
    pub fn new(id: &str, model: &str, delta: serde_json::Value, finish_reason: Option<String>) -> Self {
        Self {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created: Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta,
                finish_reason,
            }],
            usage: None,
            system_fingerprint: None,
        }
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(json: &str) -> ChatCompletionRequest {
        serde_json::from_str(json).unwrap()
    }

    // --- Deserialization ---

    #[test]
    fn test_deserialize_content_string() {
        let msg: Message = serde_json::from_str(r#"{"role":"user","content":"hello"}"#).unwrap();
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_deserialize_content_array() {
        let msg: Message = serde_json::from_str(
            r#"{"role":"user","content":[{"type":"text","text":"hello"},{"type":"text","text":"world"}]}"#,
        ).unwrap();
        assert_eq!(msg.content, "hello\nworld");
    }

    #[test]
    fn test_deserialize_stop_string() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"stop":"END"}"#);
        assert_eq!(req.stop, Some(vec!["END".to_string()]));
    }

    #[test]
    fn test_deserialize_stop_array() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"stop":["a","b"]}"#);
        assert_eq!(req.stop, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_deserialize_stop_null() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}]}"#);
        assert!(req.stop.is_none());
    }

    #[test]
    fn test_deserialize_defaults() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}]}"#);
        assert_eq!(req.temperature, Some(1.0));
        assert_eq!(req.top_p, Some(1.0));
        assert_eq!(req.n, Some(1));
        assert_eq!(req.stream, None);
        assert!(!req.is_streaming());
        assert!(!req.tools_enabled());
    }

    // --- Validation ---

    #[test]
    fn test_validate_ok() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}]}"#);
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_n_gt_1() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"n":3}"#);
        assert!(req.validate().is_err());
        assert!(req.validate().unwrap_err().contains("n > 1"));
    }

    #[test]
    fn test_validate_temperature_out_of_range() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"temperature":3.0}"#);
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_top_p_out_of_range() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"top_p":1.5}"#);
        assert!(req.validate().is_err());
    }

    // --- Sampling instructions ---

    #[test]
    fn test_sampling_default_no_instructions() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}]}"#);
        assert!(req.get_sampling_instructions().is_none());
    }

    #[test]
    fn test_sampling_low_temperature() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"temperature":0.1}"#);
        let instr = req.get_sampling_instructions().unwrap();
        assert!(instr.contains("deterministic"));
    }

    #[test]
    fn test_sampling_medium_temperature() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"temperature":0.5}"#);
        let instr = req.get_sampling_instructions().unwrap();
        assert!(instr.contains("focused"));
    }

    #[test]
    fn test_sampling_high_temperature() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"temperature":1.8}"#);
        let instr = req.get_sampling_instructions().unwrap();
        assert!(instr.contains("creative") || instr.contains("exploratory"));
    }

    #[test]
    fn test_sampling_low_top_p() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"top_p":0.3}"#);
        let instr = req.get_sampling_instructions().unwrap();
        assert!(instr.contains("probable"));
    }

    #[test]
    fn test_sampling_medium_top_p() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"top_p":0.7}"#);
        let instr = req.get_sampling_instructions().unwrap();
        assert!(instr.contains("well-established"));
    }

    // --- to_claude_options ---

    #[test]
    fn test_to_claude_options_basic() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"model":"opus"}"#);
        let opts = req.to_claude_options();
        assert_eq!(opts["model"], "opus");
        assert!(!opts.contains_key("max_thinking_tokens"));
    }

    #[test]
    fn test_to_claude_options_max_tokens() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"max_tokens":500}"#);
        let opts = req.to_claude_options();
        assert_eq!(opts["max_thinking_tokens"], 500);
    }

    #[test]
    fn test_to_claude_options_prefers_max_completion_tokens() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"max_tokens":500,"max_completion_tokens":1000}"#);
        let opts = req.to_claude_options();
        assert_eq!(opts["max_thinking_tokens"], 1000);
    }

    // --- Flags ---

    #[test]
    fn test_is_streaming() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"stream":true}"#);
        assert!(req.is_streaming());
    }

    #[test]
    fn test_tools_enabled() {
        let req = make_request(r#"{"messages":[{"role":"user","content":"hi"}],"enable_tools":true}"#);
        assert!(req.tools_enabled());
    }

    // --- Response constructors ---

    #[test]
    fn test_chat_completion_response_new() {
        let resp = ChatCompletionResponse::new("opus".into(), "hello".into(), None);
        assert!(resp.id.starts_with("chatcmpl-"));
        assert_eq!(resp.object, "chat.completion");
        assert_eq!(resp.model, "opus");
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.role, "assistant");
        assert_eq!(resp.choices[0].message.content, "hello");
        assert_eq!(resp.choices[0].finish_reason, Some("stop".to_string()));
        assert!(resp.usage.is_none());
    }

    #[test]
    fn test_chat_completion_response_with_usage() {
        let usage = Usage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 };
        let resp = ChatCompletionResponse::new("opus".into(), "hi".into(), Some(usage));
        let u = resp.usage.unwrap();
        assert_eq!(u.total_tokens, 30);
    }

    #[test]
    fn test_stream_response_new() {
        let resp = ChatCompletionStreamResponse::new(
            "id-1", "opus", serde_json::json!({"content": "hi"}), None,
        );
        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.object, "chat.completion.chunk");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn test_stream_response_with_usage() {
        let resp = ChatCompletionStreamResponse::new("id-1", "opus", serde_json::json!({}), Some("stop".into()));
        let resp = resp.with_usage(Usage { prompt_tokens: 5, completion_tokens: 10, total_tokens: 15 });
        assert!(resp.usage.is_some());
        assert_eq!(resp.usage.unwrap().total_tokens, 15);
    }
}

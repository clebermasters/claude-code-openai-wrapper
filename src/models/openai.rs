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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::openai::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTextBlock {
    #[serde(default = "default_text_type")]
    pub r#type: String,
    pub text: String,
}

fn default_text_type() -> String {
    "text".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: serde_json::Value, // String or Vec<AnthropicTextBlock>
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicMessagesRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    pub system: Option<String>,
    #[serde(default = "default_anthropic_temperature")]
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
    pub stop_sequences: Option<Vec<String>>,
    #[serde(default)]
    pub stream: Option<bool>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_anthropic_temperature() -> Option<f64> {
    Some(1.0)
}

impl AnthropicMessagesRequest {
    pub fn to_openai_messages(&self) -> Vec<Message> {
        self.messages
            .iter()
            .map(|msg| {
                let content = match &msg.content {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(blocks) => {
                        let texts: Vec<String> = blocks
                            .iter()
                            .filter_map(|block| {
                                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    block.get("text").and_then(|t| t.as_str()).map(String::from)
                                } else {
                                    None
                                }
                            })
                            .collect();
                        texts.join("\n")
                    }
                    _ => String::new(),
                };
                Message {
                    role: msg.role.clone(),
                    content,
                    name: None,
                    thinking: None,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnthropicMessagesResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<AnthropicTextBlock>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_openai_messages_string_content() {
        let req: AnthropicMessagesRequest = serde_json::from_str(r#"{
            "model": "opus",
            "messages": [
                {"role": "user", "content": "hello"},
                {"role": "assistant", "content": "hi"}
            ],
            "max_tokens": 100
        }"#).unwrap();
        let msgs = req.to_openai_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn test_to_openai_messages_block_content() {
        let req: AnthropicMessagesRequest = serde_json::from_str(r#"{
            "model": "opus",
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "part1"}, {"type": "text", "text": "part2"}]}
            ],
            "max_tokens": 100
        }"#).unwrap();
        let msgs = req.to_openai_messages();
        assert_eq!(msgs[0].content, "part1\npart2");
    }

    #[test]
    fn test_anthropic_response_new() {
        let resp = AnthropicMessagesResponse::new("opus".into(), "hi".into(), 10, 20);
        assert!(resp.id.starts_with("msg_"));
        assert_eq!(resp.r#type, "message");
        assert_eq!(resp.role, "assistant");
        assert_eq!(resp.content[0].text, "hi");
        assert_eq!(resp.model, "opus");
        assert_eq!(resp.stop_reason, Some("end_turn".to_string()));
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 20);
    }
}

impl AnthropicMessagesResponse {
    pub fn new(model: String, text: String, input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            id: format!("msg_{}", &Uuid::new_v4().to_string().replace('-', "")[..24]),
            r#type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![AnthropicTextBlock {
                r#type: "text".to_string(),
                text,
            }],
            model,
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: AnthropicUsage {
                input_tokens,
                output_tokens,
            },
        }
    }
}

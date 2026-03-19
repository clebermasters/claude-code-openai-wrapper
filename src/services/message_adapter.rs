use crate::models::openai::Message;

/// Convert OpenAI messages to Claude Code prompt format.
/// Returns (prompt, system_prompt).
pub fn messages_to_prompt(messages: &[Message]) -> (String, Option<String>) {
    let mut system_prompt = None;
    let mut conversation_parts = Vec::new();

    for message in messages {
        match message.role.as_str() {
            "system" => {
                system_prompt = Some(message.content.clone());
            }
            "user" => {
                conversation_parts.push(format!("Human: {}", message.content));
            }
            "assistant" => {
                conversation_parts.push(format!("Assistant: {}", message.content));
            }
            _ => {}
        }
    }

    let mut prompt = conversation_parts.join("\n\n");

    // If the last message wasn't from the user, add a prompt
    if let Some(last) = messages.last() {
        if last.role != "user" {
            prompt.push_str("\n\nHuman: Please continue.");
        }
    }

    (prompt, system_prompt)
}

/// Rough estimation of token count (~4 characters per token).
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4).max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_to_prompt_simple() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
            name: None,
        }];
        let (prompt, system) = messages_to_prompt(&messages);
        assert_eq!(prompt, "Human: Hello");
        assert!(system.is_none());
    }

    #[test]
    fn test_messages_to_prompt_with_system() {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are helpful.".to_string(),
                name: None,
            },
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                name: None,
            },
        ];
        let (prompt, system) = messages_to_prompt(&messages);
        assert_eq!(prompt, "Human: Hello");
        assert_eq!(system, Some("You are helpful.".to_string()));
    }

    #[test]
    fn test_messages_to_prompt_continue() {
        let messages = vec![Message {
            role: "assistant".to_string(),
            content: "Hi".to_string(),
            name: None,
        }];
        let (prompt, _) = messages_to_prompt(&messages);
        assert!(prompt.contains("Please continue."));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("Hello world!"), 3); // 12 chars / 4
        assert_eq!(estimate_tokens(""), 1); // min 1
    }
}

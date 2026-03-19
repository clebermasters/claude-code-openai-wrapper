use once_cell::sync::Lazy;
use regex::Regex;

static THINKING_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<thinking>.*?</thinking>").unwrap());

static ATTEMPT_COMPLETION_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<attempt_completion>(.*?)</attempt_completion>").unwrap());

static RESULT_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<result>(.*?)</result>").unwrap());

static TOOL_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        r"(?s)<read_file>.*?</read_file>",
        r"(?s)<write_file>.*?</write_file>",
        r"(?s)<bash>.*?</bash>",
        r"(?s)<search_files>.*?</search_files>",
        r"(?s)<str_replace_editor>.*?</str_replace_editor>",
        r"(?s)<args>.*?</args>",
        r"(?s)<ask_followup_question>.*?</ask_followup_question>",
        r"(?s)<attempt_completion>.*?</attempt_completion>",
        r"(?s)<question>.*?</question>",
        r"(?s)<follow_up>.*?</follow_up>",
        r"(?s)<suggest>.*?</suggest>",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

static IMAGE_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\[Image:.*?\]|data:image/.*?;base64,.*?(?:\s|$)").unwrap());

static MULTI_NEWLINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n\s*\n\s*\n").unwrap());

/// Filter content for unsupported features and tool usage.
/// Remove thinking blocks, tool calls, and image references.
pub fn filter_content(content: &str) -> String {
    if content.is_empty() {
        return content.to_string();
    }

    let mut result = content.to_string();

    // Remove thinking blocks
    result = THINKING_PATTERN.replace_all(&result, "").to_string();

    // Extract content from attempt_completion blocks
    if let Some(caps) = ATTEMPT_COMPLETION_PATTERN.captures(&result) {
        let extracted = caps.get(1).map_or("", |m| m.as_str()).trim();

        // If there's a <result> tag inside, extract from that
        let final_content = if let Some(result_caps) = RESULT_PATTERN.captures(extracted) {
            result_caps.get(1).map_or("", |m| m.as_str()).trim()
        } else {
            extracted
        };

        if !final_content.is_empty() {
            result = final_content.to_string();
        }
    } else {
        // Remove other tool usage blocks
        for pattern in TOOL_PATTERNS.iter() {
            result = pattern.replace_all(&result, "").to_string();
        }
    }

    // Replace image references
    result = IMAGE_PATTERN
        .replace_all(&result, "[Image: Content not supported by Claude Code]")
        .to_string();

    // Clean up extra whitespace
    result = MULTI_NEWLINE.replace_all(&result, "\n\n").to_string();
    result = result.trim().to_string();

    // Fallback for empty content
    if result.is_empty() || result.chars().all(|c| c.is_whitespace()) {
        return "I understand you're testing the system. How can I help you today?".to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_empty() {
        assert_eq!(filter_content(""), "");
    }

    #[test]
    fn test_filter_thinking_blocks() {
        let input = "Hello <thinking>internal thought</thinking> world";
        assert_eq!(filter_content(input), "Hello  world");
    }

    #[test]
    fn test_filter_attempt_completion() {
        let input = "<attempt_completion><result>Final answer</result></attempt_completion>";
        assert_eq!(filter_content(input), "Final answer");
    }

    #[test]
    fn test_filter_tool_blocks() {
        let input = "Before <bash>ls -la</bash> After";
        assert_eq!(filter_content(input), "Before  After");
    }

    #[test]
    fn test_filter_whitespace_only() {
        let input = "   \n\n   ";
        assert_eq!(
            filter_content(input),
            "I understand you're testing the system. How can I help you today?"
        );
    }

    #[test]
    fn test_filter_image_refs() {
        let input = "See [Image: screenshot.png] here";
        assert_eq!(
            filter_content(input),
            "See [Image: Content not supported by Claude Code] here"
        );
    }
}

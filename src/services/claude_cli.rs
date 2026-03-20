use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use crate::config::Config;

/// Claude CLI subprocess integration.
/// Replaces the Python claude-agent-sdk with direct CLI calls.
#[derive(Clone)]
pub struct ClaudeCli {
    config: Arc<Config>,
    cwd: PathBuf,
    auth_env_vars: HashMap<String, String>,
    cli_path: String,
}

impl ClaudeCli {
    pub fn new(config: Arc<Config>, auth_env_vars: HashMap<String, String>) -> Result<Self, String> {
        let cwd = if let Some(ref cwd_str) = config.claude_cwd {
            let path = PathBuf::from(cwd_str);
            if !path.exists() {
                return Err(format!("Working directory does not exist: {}", cwd_str));
            }
            info!("Using CLAUDE_CWD: {}", cwd_str);
            path
        } else {
            let tmp = std::env::temp_dir().join(format!("claude_code_workspace_{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&tmp)
                .map_err(|e| format!("Failed to create temp workspace: {e}"))?;
            info!("Using temporary isolated workspace: {}", tmp.display());
            tmp
        };

        Ok(Self {
            cli_path: config.claude_cli_path.clone(),
            config,
            cwd,
            auth_env_vars,
        })
    }

    /// Verify Claude CLI is accessible and working.
    pub async fn verify_cli(&self) -> bool {
        info!("Testing Claude CLI...");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.run_command(&["--version"]),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                info!("Claude CLI verified: {}", output.trim());
                true
            }
            Ok(Err(e)) => {
                error!("Claude CLI verification failed: {e}");
                warn!("Please ensure Claude Code is installed: npm install -g @anthropic-ai/claude-code");
                false
            }
            Err(_) => {
                warn!("Claude CLI verification timed out (30s)");
                false
            }
        }
    }

    /// Run a simple CLI command and return stdout.
    async fn run_command(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.cli_path)
            .args(args)
            .current_dir(&self.cwd)
            .envs(&self.auth_env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Claude CLI exited with error: {stderr}"))
        }
    }

    /// Build the full CLI arguments for a completion.
    fn build_args(&self, prompt: &str, opts: &CliOptions, stream_json: bool) -> Vec<String> {
        let mut args = vec![
            "--print".to_string(),
            "--output-format".to_string(),
            if stream_json { "stream-json".to_string() } else { "json".to_string() },
        ];

        if stream_json {
            args.push("--verbose".to_string());
        }

        if let Some(ref m) = opts.model {
            args.push("--model".to_string());
            args.push(m.clone());
        }

        if let Some(ref fm) = opts.fallback_model {
            args.push("--fallback-model".to_string());
            args.push(fm.clone());
        }

        let turns = opts.max_turns.unwrap_or(self.config.cli_max_turns);
        if turns > 0 {
            args.push("--max-turns".to_string());
            args.push(turns.to_string());
        }

        if let Some(budget) = opts.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(budget.to_string());
        }

        if let Some(ref e) = opts.effort {
            args.push("--effort".to_string());
            args.push(e.clone());
        }

        if let Some(ref sp) = opts.system_prompt {
            args.push("--system-prompt".to_string());
            args.push(sp.clone());
        }

        if let Some(ref asp) = opts.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(asp.clone());
        }

        if let Some(ref schema) = opts.json_schema {
            args.push("--json-schema".to_string());
            args.push(schema.clone());
        }

        if let Some(ref tools) = opts.allowed_tools {
            if !tools.is_empty() {
                args.push("--allowedTools".to_string());
                args.push(tools.join(","));
            }
        }

        if let Some(ref tools) = opts.disallowed_tools {
            if !tools.is_empty() {
                let all_tools: std::collections::HashSet<&str> =
                    crate::constants::CLAUDE_TOOLS.iter().copied().collect();
                let disallowed_set: std::collections::HashSet<&str> =
                    tools.iter().map(|s| s.as_str()).collect();
                if disallowed_set != all_tools {
                    args.push("--disallowedTools".to_string());
                    args.push(tools.join(","));
                }
            }
        }

        if let Some(ref pm) = opts.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(pm.clone());
        }

        args.push(prompt.to_string());
        args
    }

    /// Non-streaming completion: run CLI and return the full response text.
    /// When `include_thinking` is true, uses stream-json internally to capture
    /// thinking blocks (the json format strips them).
    pub async fn run_completion(
        &self,
        prompt: &str,
        opts: &CliOptions,
    ) -> Result<CompletionResult, String> {
        if opts.include_thinking {
            return self.run_completion_with_thinking(prompt, opts).await;
        }

        let args = self.build_args(prompt, opts, false);
        debug!("Running claude CLI with args: {:?}", args);

        let timeout = std::time::Duration::from_millis(self.config.max_timeout_ms);

        let result = tokio::time::timeout(timeout, async {
            let output = Command::new(&self.cli_path)
                .args(&args)
                .current_dir(&self.cwd)
                .envs(&self.auth_env_vars)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Failed to run claude CLI: {e}"))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if !stderr.is_empty() {
                debug!("Claude CLI stderr: {stderr}");
            }

            if !output.status.success() && stdout.is_empty() {
                return Err(format!("Claude CLI error: {stderr}"));
            }

            Ok(stdout)
        })
        .await
        .map_err(|_| "Claude CLI timed out".to_string())?;

        let stdout = result?;
        let text = self.extract_result_text(&stdout);
        let metadata = self.extract_metadata_from_output(&stdout);

        Ok(CompletionResult {
            text: text.unwrap_or_default(),
            thinking: None,
            metadata,
        })
    }

    /// Run completion using stream-json internally to capture thinking blocks.
    async fn run_completion_with_thinking(
        &self,
        prompt: &str,
        opts: &CliOptions,
    ) -> Result<CompletionResult, String> {
        let mut rx = self.run_completion_stream(prompt, opts).await?;

        let mut text_parts = Vec::new();
        let mut thinking_parts = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::AssistantText(t) => text_parts.push(t),
                StreamEvent::Thinking(t) => thinking_parts.push(t),
                StreamEvent::Result(t) => {
                    if text_parts.is_empty() {
                        text_parts.push(t);
                    }
                }
                StreamEvent::Error(e) => return Err(e),
                StreamEvent::Done => break,
                _ => {}
            }
        }

        Ok(CompletionResult {
            text: text_parts.join(""),
            thinking: if thinking_parts.is_empty() { None } else { Some(thinking_parts.join("\n")) },
            metadata: CompletionMetadata::default(),
        })
    }

    /// Streaming completion: run CLI with stream-json and return lines via a channel.
    pub async fn run_completion_stream(
        &self,
        prompt: &str,
        opts: &CliOptions,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, String> {
        let args = self.build_args(prompt, opts, true);

        debug!("Running claude CLI (streaming) with args: {:?}", args);

        let mut child = Command::new(&self.cli_path)
            .args(&args)
            .current_dir(&self.cwd)
            .envs(&self.auth_env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn claude CLI: {e}"))?;

        let stdout = child.stdout.take().ok_or("No stdout from CLI")?;
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        let timeout_ms = self.config.max_timeout_ms;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            let deadline = tokio::time::Instant::now() + timeout;

            loop {
                let read_result = tokio::time::timeout_at(deadline, lines.next_line()).await;

                match read_result {
                    Ok(Ok(Some(line))) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        debug!("CLI stream line: {}", &line[..line.len().min(200)]);

                        match serde_json::from_str::<serde_json::Value>(&line) {
                            Ok(json) => {
                                let events = parse_stream_event(&json);
                                for event in events {
                                    if tx.send(event).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Non-JSON line from CLI: {e}");
                                // Send as raw text
                                if tx.send(StreamEvent::Text(line)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => break, // EOF
                    Ok(Err(e)) => {
                        error!("Error reading CLI stdout: {e}");
                        let _ = tx.send(StreamEvent::Error(e.to_string())).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx.send(StreamEvent::Error("Claude CLI timed out".to_string())).await;
                        break;
                    }
                }
            }

            let _ = tx.send(StreamEvent::Done).await;
            let _ = child.wait().await;
        });

        Ok(rx)
    }

    /// Extract the result text from JSON output.
    fn extract_result_text(&self, output: &str) -> Option<String> {
        // Try to parse as a single JSON object first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            // Check for "result" field (ResultMessage format)
            if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                return Some(result.to_string());
            }
            // Check for content array
            if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
                let texts: Vec<&str> = content
                    .iter()
                    .filter_map(|block| {
                        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                            block.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();
                if !texts.is_empty() {
                    return Some(texts.join("\n"));
                }
            }
        }

        // Try parsing as NDJSON (multiple JSON objects per line)
        let mut last_text = None;
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                // ResultMessage with result
                if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
                    if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                        return Some(result.to_string());
                    }
                }
                // AssistantMessage: {"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}}
                if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                    if let Some(msg) = json.get("message") {
                        if let Some(text) = extract_text_from_content(msg.get("content")) {
                            last_text = Some(text);
                        }
                    }
                }
                // Direct content array
                if let Some(text) = extract_text_from_content(json.get("content")) {
                    last_text = Some(text);
                }
            }
        }

        last_text
    }

    /// Extract thinking text from CLI output (mirrors extract_result_text but for thinking blocks).
    fn extract_thinking_text(&self, output: &str) -> Option<String> {
        // Single JSON object
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(thinking) = extract_thinking_from_content(json.get("content")) {
                return Some(thinking);
            }
        }

        // NDJSON: scan for assistant messages with thinking blocks
        let mut last_thinking = None;
        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                    if let Some(msg) = json.get("message") {
                        if let Some(thinking) = extract_thinking_from_content(msg.get("content")) {
                            last_thinking = Some(thinking);
                        }
                    }
                }
                if let Some(thinking) = extract_thinking_from_content(json.get("content")) {
                    last_thinking = Some(thinking);
                }
            }
        }

        last_thinking
    }

    fn extract_metadata_from_output(&self, output: &str) -> CompletionMetadata {
        let mut metadata = CompletionMetadata::default();

        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
                    metadata.total_cost_usd = json.get("total_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    metadata.duration_ms = json.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    metadata.num_turns = json.get("num_turns").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    metadata.session_id = json.get("session_id").and_then(|v| v.as_str()).map(String::from);
                }
                if json.get("subtype").and_then(|s| s.as_str()) == Some("init") {
                    if let Some(data) = json.get("data") {
                        metadata.session_id = data.get("session_id").and_then(|v| v.as_str()).map(String::from);
                        metadata.model = data.get("model").and_then(|v| v.as_str()).map(String::from);
                    }
                }
            }
        }

        metadata
    }

    /// Estimate token usage based on character count (~4 chars/token).
    pub fn estimate_token_usage(&self, prompt: &str, completion: &str) -> (u32, u32) {
        let prompt_tokens = (prompt.len() / 4).max(1) as u32;
        let completion_tokens = (completion.len() / 4).max(1) as u32;
        (prompt_tokens, completion_tokens)
    }
}

/// Options for CLI invocation. Bundles all optional flags to avoid
/// positional parameter explosion as we add new CLI features.
#[derive(Debug, Clone, Default)]
pub struct CliOptions {
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub permission_mode: Option<String>,
    pub max_turns: Option<u32>,
    pub effort: Option<String>,
    pub include_thinking: bool,
    pub max_budget_usd: Option<f64>,
    pub fallback_model: Option<String>,
    pub json_schema: Option<String>,
    pub append_system_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionMetadata {
    pub session_id: Option<String>,
    pub total_cost_usd: f64,
    pub duration_ms: u64,
    pub num_turns: u32,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionResult {
    pub text: String,
    pub thinking: Option<String>,
    pub metadata: CompletionMetadata,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Text(String),
    AssistantText(String),
    Thinking(String),
    Result(String),
    Error(String),
    Done,
}

/// Extract text from a content array (handles [{"type":"text","text":"..."},...])
fn extract_text_from_content(content: Option<&serde_json::Value>) -> Option<String> {
    let arr = content?.as_array()?;
    let texts: Vec<&str> = arr
        .iter()
        .filter_map(|block| {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                block.get("text").and_then(|t| t.as_str())
            } else {
                None
            }
        })
        .collect();
    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

/// Extract thinking text from a content array (handles [{"type":"thinking","thinking":"..."},...])
fn extract_thinking_from_content(content: Option<&serde_json::Value>) -> Option<String> {
    let arr = content?.as_array()?;
    let thoughts: Vec<&str> = arr
        .iter()
        .filter_map(|block| {
            if block.get("type").and_then(|t| t.as_str()) == Some("thinking") {
                block.get("thinking").and_then(|t| t.as_str())
            } else {
                None
            }
        })
        .collect();
    if thoughts.is_empty() {
        None
    } else {
        Some(thoughts.join("\n"))
    }
}

fn parse_stream_event(json: &serde_json::Value) -> Vec<StreamEvent> {
    let mut events = Vec::new();

    // ResultMessage with result text
    if json.get("subtype").and_then(|s| s.as_str()) == Some("success") {
        if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
            return vec![StreamEvent::Result(result.to_string())];
        }
    }

    // AssistantMessage: {"type":"assistant","message":{"content":[...]}}
    if json.get("type").and_then(|t| t.as_str()) == Some("assistant") {
        if let Some(msg) = json.get("message") {
            if let Some(thinking) = extract_thinking_from_content(msg.get("content")) {
                events.push(StreamEvent::Thinking(thinking));
            }
            if let Some(text) = extract_text_from_content(msg.get("content")) {
                events.push(StreamEvent::AssistantText(text));
            }
            if !events.is_empty() {
                return events;
            }
        }
    }

    // Direct content array fallback
    if let Some(thinking) = extract_thinking_from_content(json.get("content")) {
        events.push(StreamEvent::Thinking(thinking));
    }
    if let Some(text) = extract_text_from_content(json.get("content")) {
        events.push(StreamEvent::AssistantText(text));
    }
    if !events.is_empty() {
        return events;
    }

    // Error in result
    if json.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
        let msg = json.get("result")
            .or_else(|| json.get("error_message"))
            .or_else(|| json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return vec![StreamEvent::Error(msg.to_string())];
    }

    // Default: skip system/rate_limit messages
    vec![StreamEvent::Text(String::new())]
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_text_from_content ---

    #[test]
    fn test_extract_text_from_content_none() {
        assert!(extract_text_from_content(None).is_none());
    }

    #[test]
    fn test_extract_text_from_content_non_array() {
        let val = serde_json::json!("just a string");
        assert!(extract_text_from_content(Some(&val)).is_none());
    }

    #[test]
    fn test_extract_text_from_content_text_block() {
        let val = serde_json::json!([{"type": "text", "text": "hello"}]);
        assert_eq!(extract_text_from_content(Some(&val)).unwrap(), "hello");
    }

    #[test]
    fn test_extract_text_from_content_multiple_blocks() {
        let val = serde_json::json!([
            {"type": "text", "text": "line1"},
            {"type": "tool_use", "name": "Bash"},
            {"type": "text", "text": "line2"}
        ]);
        assert_eq!(extract_text_from_content(Some(&val)).unwrap(), "line1\nline2");
    }

    #[test]
    fn test_extract_text_from_content_no_text_blocks() {
        let val = serde_json::json!([{"type": "tool_use", "name": "Bash"}]);
        assert!(extract_text_from_content(Some(&val)).is_none());
    }

    // --- extract_thinking_from_content ---

    #[test]
    fn test_extract_thinking_none() {
        assert!(extract_thinking_from_content(None).is_none());
    }

    #[test]
    fn test_extract_thinking_no_thinking_blocks() {
        let val = serde_json::json!([{"type": "text", "text": "hello"}]);
        assert!(extract_thinking_from_content(Some(&val)).is_none());
    }

    #[test]
    fn test_extract_thinking_single_block() {
        let val = serde_json::json!([
            {"type": "thinking", "thinking": "Let me reason about this..."},
            {"type": "text", "text": "answer"}
        ]);
        assert_eq!(
            extract_thinking_from_content(Some(&val)).unwrap(),
            "Let me reason about this..."
        );
    }

    #[test]
    fn test_extract_thinking_multiple_blocks() {
        let val = serde_json::json!([
            {"type": "thinking", "thinking": "Step 1"},
            {"type": "thinking", "thinking": "Step 2"},
            {"type": "text", "text": "answer"}
        ]);
        assert_eq!(
            extract_thinking_from_content(Some(&val)).unwrap(),
            "Step 1\nStep 2"
        );
    }

    // --- extract_thinking_text (NDJSON) ---

    #[test]
    fn test_extract_thinking_text_ndjson() {
        let cli = make_cli();
        let output = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"reasoning here"},{"type":"text","text":"answer"}]}}"#;
        assert_eq!(cli.extract_thinking_text(output).unwrap(), "reasoning here");
    }

    #[test]
    fn test_extract_thinking_text_no_thinking() {
        let cli = make_cli();
        let output = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"just text"}]}}"#;
        assert!(cli.extract_thinking_text(output).is_none());
    }

    // --- parse_stream_event ---

    #[test]
    fn test_parse_stream_event_result() {
        let json = serde_json::json!({"subtype": "success", "result": "answer"});
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Result(text) => assert_eq!(text, "answer"),
            other => panic!("Expected Result, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_assistant() {
        let json = serde_json::json!({
            "type": "assistant",
            "message": {"content": [{"type": "text", "text": "hi there"}]}
        });
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::AssistantText(text) => assert_eq!(text, "hi there"),
            other => panic!("Expected AssistantText, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_assistant_with_thinking() {
        let json = serde_json::json!({
            "type": "assistant",
            "message": {"content": [
                {"type": "thinking", "thinking": "Let me think about this..."},
                {"type": "text", "text": "The answer is 42."}
            ]}
        });
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::Thinking(t) => assert_eq!(t, "Let me think about this..."),
            other => panic!("Expected Thinking, got {:?}", other),
        }
        match &events[1] {
            StreamEvent::AssistantText(t) => assert_eq!(t, "The answer is 42."),
            other => panic!("Expected AssistantText, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_direct_content() {
        let json = serde_json::json!({"content": [{"type": "text", "text": "direct"}]});
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::AssistantText(text) => assert_eq!(text, "direct"),
            other => panic!("Expected AssistantText, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_error() {
        let json = serde_json::json!({"is_error": true, "result": "something broke"});
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Error(msg) => assert_eq!(msg, "something broke"),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_error_fallback() {
        let json = serde_json::json!({"is_error": true, "error_message": "fail"});
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Error(msg) => assert_eq!(msg, "fail"),
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_stream_event_system_ignored() {
        let json = serde_json::json!({"type": "system", "subtype": "init"});
        let events = parse_stream_event(&json);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::Text(t) => assert!(t.is_empty()),
            other => panic!("Expected empty Text, got {:?}", other),
        }
    }

    // --- extract_result_text ---

    fn make_cli() -> ClaudeCli {
        let config = std::sync::Arc::new(crate::config::Config::from_env());
        ClaudeCli {
            cli_path: "claude".into(),
            config,
            cwd: std::path::PathBuf::from("/tmp"),
            auth_env_vars: HashMap::new(),
        }
    }

    #[test]
    fn test_extract_result_text_single_json() {
        let cli = make_cli();
        let output = r#"{"result": "Four."}"#;
        assert_eq!(cli.extract_result_text(output), Some("Four.".into()));
    }

    #[test]
    fn test_extract_result_text_content_array() {
        let cli = make_cli();
        let output = r#"{"content": [{"type": "text", "text": "Hello world"}]}"#;
        assert_eq!(cli.extract_result_text(output), Some("Hello world".into()));
    }

    #[test]
    fn test_extract_result_text_ndjson() {
        let cli = make_cli();
        let output = r#"{"type":"system","subtype":"init"}
{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}
{"subtype":"success","result":"Hello"}"#;
        assert_eq!(cli.extract_result_text(output), Some("Hello".into()));
    }

    #[test]
    fn test_extract_result_text_empty() {
        let cli = make_cli();
        assert!(cli.extract_result_text("").is_none());
    }

    #[test]
    fn test_extract_result_text_invalid_json() {
        let cli = make_cli();
        assert!(cli.extract_result_text("not json at all").is_none());
    }

    // --- extract_metadata ---

    #[test]
    fn test_extract_metadata_success() {
        let cli = make_cli();
        let output = r#"{"subtype":"success","total_cost_usd":0.05,"duration_ms":1234,"num_turns":2,"session_id":"abc-123"}"#;
        let meta = cli.extract_metadata_from_output(output);
        assert_eq!(meta.total_cost_usd, 0.05);
        assert_eq!(meta.duration_ms, 1234);
        assert_eq!(meta.num_turns, 2);
        assert_eq!(meta.session_id, Some("abc-123".into()));
    }

    #[test]
    fn test_extract_metadata_init() {
        let cli = make_cli();
        let output = r#"{"subtype":"init","data":{"session_id":"sid-1","model":"opus"}}"#;
        let meta = cli.extract_metadata_from_output(output);
        assert_eq!(meta.session_id, Some("sid-1".into()));
        assert_eq!(meta.model, Some("opus".into()));
    }

    #[test]
    fn test_extract_metadata_empty() {
        let cli = make_cli();
        let meta = cli.extract_metadata_from_output("");
        assert!(meta.session_id.is_none());
        assert_eq!(meta.total_cost_usd, 0.0);
    }

    // --- estimate_token_usage ---

    #[test]
    fn test_estimate_tokens() {
        let cli = make_cli();
        let (p, c) = cli.estimate_token_usage("Hello world!", "Hi");
        assert_eq!(p, 3); // 12 / 4
        assert_eq!(c, 1); // min 1
    }

    #[test]
    fn test_estimate_tokens_empty() {
        let cli = make_cli();
        let (p, c) = cli.estimate_token_usage("", "");
        assert_eq!(p, 1); // min 1
        assert_eq!(c, 1);
    }

    // --- build_args ---

    fn opts() -> CliOptions {
        CliOptions::default()
    }

    #[test]
    fn test_build_args_basic() {
        let cli = make_cli();
        let o = CliOptions { model: Some("opus".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--print".into()));
        assert!(args.contains(&"json".into()));
        assert!(args.contains(&"--model".into()));
        assert!(args.contains(&"opus".into()));
        assert_eq!(args.last().unwrap(), "hello");
        assert!(!args.contains(&"--verbose".into()));
    }

    #[test]
    fn test_build_args_stream_json() {
        let cli = make_cli();
        let args = cli.build_args("hello", &opts(), true);
        assert!(args.contains(&"stream-json".into()));
        assert!(args.contains(&"--verbose".into()));
    }

    #[test]
    fn test_build_args_system_prompt() {
        let cli = make_cli();
        let o = CliOptions { system_prompt: Some("be helpful".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        let idx = args.iter().position(|a| a == "--system-prompt").unwrap();
        assert_eq!(args[idx + 1], "be helpful");
    }

    #[test]
    fn test_build_args_allowed_tools() {
        let cli = make_cli();
        let o = CliOptions { allowed_tools: Some(vec!["Read".into(), "Write".into()]), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--allowedTools".into()));
        assert!(args.contains(&"Read,Write".into()));
    }

    #[test]
    fn test_build_args_permission_mode() {
        let cli = make_cli();
        let o = CliOptions { permission_mode: Some("bypassPermissions".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--permission-mode".into()));
        assert!(args.contains(&"bypassPermissions".into()));
    }

    #[test]
    fn test_build_args_all_tools_disabled_skips_flag() {
        let cli = make_cli();
        let all: Vec<String> = crate::constants::CLAUDE_TOOLS.iter().map(|s| s.to_string()).collect();
        let o = CliOptions { disallowed_tools: Some(all), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(!args.contains(&"--disallowedTools".into()));
    }

    #[test]
    fn test_build_args_partial_disallowed() {
        let cli = make_cli();
        let o = CliOptions { disallowed_tools: Some(vec!["Task".into(), "WebFetch".into()]), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--disallowedTools".into()));
        assert!(args.contains(&"Task,WebFetch".into()));
    }

    #[test]
    fn test_build_args_max_turns() {
        let cli = make_cli();
        let o = CliOptions { max_turns: Some(5), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--max-turns".into()));
        assert!(args.contains(&"5".into()));
    }

    #[test]
    fn test_build_args_max_turns_zero_omitted() {
        let cli = make_cli();
        let o = CliOptions { max_turns: Some(0), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(!args.contains(&"--max-turns".into()));
    }

    #[test]
    fn test_build_args_effort() {
        let cli = make_cli();
        let o = CliOptions { effort: Some("max".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--effort".into()));
        assert!(args.contains(&"max".into()));
    }

    #[test]
    fn test_build_args_effort_none_omitted() {
        let cli = make_cli();
        let args = cli.build_args("hello", &opts(), false);
        assert!(!args.contains(&"--effort".into()));
    }

    #[test]
    fn test_build_args_max_budget_usd() {
        let cli = make_cli();
        let o = CliOptions { max_budget_usd: Some(5.5), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--max-budget-usd".into()));
        assert!(args.contains(&"5.5".into()));
    }

    #[test]
    fn test_build_args_fallback_model() {
        let cli = make_cli();
        let o = CliOptions { fallback_model: Some("claude-haiku-4-5-20251001".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--fallback-model".into()));
        assert!(args.contains(&"claude-haiku-4-5-20251001".into()));
    }

    #[test]
    fn test_build_args_json_schema() {
        let cli = make_cli();
        let schema = r#"{"type":"object"}"#.to_string();
        let o = CliOptions { json_schema: Some(schema.clone()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--json-schema".into()));
        assert!(args.contains(&schema));
    }

    #[test]
    fn test_build_args_append_system_prompt() {
        let cli = make_cli();
        let o = CliOptions { append_system_prompt: Some("Always respond in JSON".into()), ..opts() };
        let args = cli.build_args("hello", &o, false);
        assert!(args.contains(&"--append-system-prompt".into()));
        assert!(args.contains(&"Always respond in JSON".into()));
    }

    #[test]
    fn test_build_args_defaults_omit_optional() {
        let cli = make_cli();
        let args = cli.build_args("hello", &opts(), false);
        assert!(!args.contains(&"--effort".into()));
        assert!(!args.contains(&"--max-budget-usd".into()));
        assert!(!args.contains(&"--fallback-model".into()));
        assert!(!args.contains(&"--json-schema".into()));
        assert!(!args.contains(&"--append-system-prompt".into()));
        assert!(!args.contains(&"--max-turns".into()));
    }
}
